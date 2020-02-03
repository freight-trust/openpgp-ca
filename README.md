# OpenPGP certification authority

OpenPGP CA is a tool for managing OpenPGP keys within an organization.

The primary goal is to make it trivial for end users to authenticate
OpenPGP keys for users in their organization or in adjacent organizations.

OpenPGP CA is built using https://gitlab.com/sequoia-pgp/sequoia


## Building

OpenPGP CA requires:

- Rust and Cargo, see https://www.rust-lang.org/tools/install

- the C-dependencies of Sequoia PGP, see "Building Sequoia" at https://gitlab.com/sequoia-pgp/sequoia

Then run `cargo build --release` - the resulting binary is at `target/release/openpgp-ca`  

It's possible to run OpenPGP CA in Docker, [see below](#running-in-docker).

## General operation

### Database

OpenPGP CA uses an sqlite database to keep all of its state.

There are 3 ways of configuring while database file is user:

1.  the most common way is to set the `OPENPGP_CA_DB` environment variable
2.  the optional parameter "-d" overrides all other settings and sets the database file
3.  a `.env` file can set the environment variable `DATABASE_URL` "in the style of the ruby dotenv gem"

If the configured database file doesn't exist, it will get created implicitly.


### Help

The parameter "--help" will give information on any command level, e.g.

`openpgp-ca --help`

or 

`openpgp-ca user --help`

or

`openpgp-ca user import --help`


## Decentralized key creation workflow (user keys get generated on user machines, not by OpenPGP CA)

### (1) OpenPGP CA: set up, export CA public key

*  Set environment variable to configure where the database is stored:
 
`export OPENPGP_CA_DB=/tmp/openpgp-ca.sqlite`

*  Set up a new CA instance and generate a new keypair for the CA:

`openpgp-ca ca new example.org` 

*  Export the CA public key, for use on client machines:

`openpgp-ca ca export > ca.pubkey` 

### (2) On user machine using gpg: import CA public key, create new user

*  Set up a gpg test environment and import the CA public key:

`mkdir /tmp/test/`

`export GNUPGHOME=/tmp/test/`

`gpg --import ca.pubkey`

*  create and export a keypair (and optionally a revocation certificate) for
 Alice:

`gpg --quick-generate-key alice@example.org`

`gpg --export --armor alice@example.org > alice.pubkey`

`gpg --gen-revoke alice@example.org > alice-revocation.asc`

Alternatively, if your `gpg` generated a revocation certificate automagically (usually in `$GNUPGHOME/openpgp-revocs.d/<key_fingerprint>.rev`), you can use that, but remember to edit the file and remove the "`:`" at the beginning of the "`BEGIN PGP PUBLIC KEY BLOCK`" line.

*  tsign the CA public key with this key:

`gpg --edit-key openpgp-ca@example.org`

enter `tsign`, `2`, `250`, no domain (so just hit `Enter`), `y`, `save`.

*  export the signed CA public key:

`gpg --export --armor openpgp-ca@example.org > ca-tsigned.pubkey`

### (3) OpenPGP CA: import newly created user

*  copy the files `ca-tsigned.pubkey`, `alice.pubkey` and
 `alice-revocation.asc` so they are accessible for OpenPGP CA 

*  In OpenPGP CA, import Alice's key and revocation certificate - and Alice's
 trust signature on the CA key:

`openpgp-ca user import --name "Alice Adams" --email alice@example.org --key-file alice.pubkey --revocation-file alice-revocation.asc`

`openpgp-ca ca import-tsig --file ca-tsigned.pubkey`

*  Check OpenPGP CA's user list:

`openpgp-ca user list`

This should show that Alice's key has been signed by the CA and that Alice
 has made a trust signature on the CA public key  

*  Export Alice's public key (this includes the signature by the CA):

`openpgp-ca user export --email alice@example.org`


## Centralized key creation workflow (user keys get generated by OpenPGP CA)

### (1) OpenPGP CA:
#### set up CA

*  Set environment variable to configure where the database is stored:
 
`export OPENPGP_CA_DB=/tmp/openpgp-ca.sqlite`

*  Set up a new CA instance and generate a new keypair for the CA:

`openpgp-ca ca new example.org` 

#### create new user

`openpgp-ca user add --email alice@example.org --name "Alice Adams"`

The new user's private Key is shown as output of this command, but not
stored. It needs to be copied to the user's devices and imported into the
OpenPGP keystore there. We're going to paste the key into a file
`alice.privatekey` for this example.

#### export CA public key

*  Export the CA public key, for use on client machines (the key is tsigned
 by Alice at this point):

`openpgp-ca ca export > ca.pubkey` 

### (2) on user machine using gpg: import CA public key, user private key

*  Set up a gpg test environment and import the CA public key:

`mkdir /tmp/test/`

`export GNUPGHOME=/tmp/test/`

* Import user private key

`gpg --import alice.privatekey`

* Set ownertrust for this key

`gpg --edit-key alice@example.org`

Then `trust`, `5`, `quit`.

* Import CA public key

`gpg --import ca.pubkey`

* gpg now shows the Key for alice with "ultimate" trust, and the ca Key
 with "full" trust:
 
`gpg --list-keys` 


## Workflow: Bridging of two OpenPGP CA instances

This workflow builds on the "centralized key creation" workflow from above.

Two independent instances of OpenPGP CA are set up, users are created in each
instance. Then a "bridge" is configured between both OpenPGP CA instances.

Such a bridge is configured when the CA Admins at both organizations are
satisfied that the CA Admin of the other organization is following good
procedures in signing keys of users within their organization.

The end result is that users can seamlessly authenticate users in the
other organization, and vice versa.

### (1) OpenPGP CA instance 1 (setup CA and create a user)

set up CA, create a user

`export OPENPGP_CA_DB=/tmp/openpgp-ca1.sqlite`

`openpgp-ca ca new some.org`

`openpgp-ca user add --email alice@some.org --name "Alice Adams"`

export public PGP Certificate of OpenPGP CA admin:

`openpgp-ca ca export > ca1.pub`

### (2) OpenPGP CA instance 2 (setup CA and create a user)

`export OPENPGP_CA_DB=/tmp/openpgp-ca2.sqlite`

`openpgp-ca ca new other.org`

`openpgp-ca user add --email bob@other.org --name "Bob Baker"`

export public PGP Certificate of OpenPGP CA admin:

`openpgp-ca ca export > ca2.pub`

### (3) OpenPGP CA instance 1 (configure bridge to instance 2, export keys)

`export OPENPGP_CA_DB=/tmp/openpgp-ca1.sqlite`

CA 1 creates a trust signature for the public key of CA 2 (implicitly
scoped to the domainname "other.org") of the remote organization

`openpgp-ca bridge new --remote-key-file ca2.pub`

OpenPGP CA prints a message showing the fingerprint of the remote key
that you just configured a bridge to. Please double-check that this
fingerprint really belongs to the intended remote CA before disseminating
the newly trust-signed public key!

Export signed public key of CA 2:

`openpgp-ca bridge list > ca2.signed`

Export user keys

`openpgp-ca user export > ca1.users`

### (4) OpenPGP CA instance 2 (configure bridge to instance 1, export keys)

`export OPENPGP_CA_DB=/tmp/openpgp-ca2.sqlite`

CA 2 creates a trust signature for the public key of CA 1 (implicitly
scoped to the domainname "some.org") of the remote organization (again,
please make sure that the fingerprint belongs to the intended remote CA!)

`openpgp-ca bridge new --remote-key-file ca1.pub`

Export signed public key of CA 1:

`openpgp-ca bridge list > ca1.signed`

Export user keys

`openpgp-ca user export > ca2.users`

### (5) Import all keys into "Alice" gnupg test environment, confirm authentication

`mkdir /tmp/test/ && export GNUPGHOME=/tmp/test/`

`gpg --import  ca1.signed  ca2.signed ca1.users ca2.users`

Set ownertrust for Alice:

`gpg --edit-key alice@some.org`

Then `trust`, `5`, `quit`.

The resulting situation is what Alice (who works at "some.org") would see in
her OpenPGP instance:

gpg shows "ultimate" trust for Alice's own key, and "full" trust for
both OpenPGP CA Admin keys, as well as Bob (who works at "other.org"):

`gpg --list-keys`

## Variation on the bridging Workflow example:

In step (2), CA 2 creates an additional user outside of the domain "other.org":

`openpgp-ca user add --email carol@third.org --name "Carol Cruz"`

The rest of the workflow is performed exactly as above.

Alice can still authenticate both OpenPGP CA admin Certificates, as well as
Bob. Carol however is (correctly) shown as not authenticated.


## Workflows for exporting Certificates from OpenPGP CA

Set up a new OpenPGP CA instance with two users: 

`export OPENPGP_CA_DB=/tmp/openpgp-ca.sqlite`

`openpgp-ca ca new example.org` 

`openpgp-ca user add --email alice@example.org --name "Alice Adams"`

`openpgp-ca user add --email bob@example.org --name "Bob Baker"`


## Inspecting user certificates in OpenPGP CA

We can inspect the state of the users in OpenPGP CA like this:

`openpgp-ca user list`

Exporting an individual user certificate (the armorded certificate will be
printed on stdout):

`openpgp-ca user export -e alice@example.org`

To output all public certificates from OpenPGP:

`openpgp-ca user export`

To output the public certificate of the OpenPGP CA admin:

`openpgp-ca ca export`

OpenPGP CA can check if all keys are mutually signed (user keys tsigned the
 CA key, and the CA key has signed the user key), and report the results:
 
`openpgp-ca user check sigs`
 
OpenPGP CA can check if any keys have expired, and report the results:
 
`openpgp-ca user check expiry`

OpenPGP CA can also check if any keys have expired a specified number of
 days in the future and report the results:
 
`openpgp-ca user check expiry --days 60`
  
  
## Dealing with revocations of user certificates in OpenPGP CA

Check which revocation certificates exist for a given email.

`openpgp-ca user show-revocations --email bob@example.org`

The results show a numeric "revocation id".

Apply a revocation to the user's certificate:

`openpgp-ca user apply-revocation --id 2`

Afterwards, "show-revocations" will display the additional note: "this
revocation has been PUBLISHED", and the user's public key contains the
revocation certificate.

The updated public key can be displayed by running
 
`openpgp-ca user export --email 'bob@example.org'`

## Workflow: Export Certificates to a Web Key Directory (WKD)

OpenPGP CA can export Certificates in Web Key Directory (WKD) format
(https://tools.ietf.org/html/draft-koch-openpgp-webkey-service-08)

Set up a new OpenPGP CA instance and create two users: 

`export OPENPGP_CA_DB=/tmp/openpgp-ca.sqlite`

`openpgp-ca ca new example.org` 

`openpgp-ca user add --email alice@example.org --name "Alice Adams"`

`openpgp-ca user add --email bob@example.org --name "Bob Baker"`

Export keys into a WKD structure:

`openpgp-ca wkd export /tmp/wkd/`

Using/testing WKD as a client (to use WKD, the export needs to be on the
webserver for the relevant domain, in the correct directory, with https set
up):

`gpg --auto-key-locate clear,nodefault,wkd --locate-key openpgp-ca@example.org`

or

`sq wkd get openpgp-ca@example.org`


## Some usage examples using cargo to run openpgp-ca:

```
cargo run ca new example.org
cargo run -- -d /tmp/ca.sqlite ca new example.org

cargo run user add --email alice@example.org --email a@example.org --name "Alice Adams"
```

## Running in Docker

You can also use `openpgp-ca` in [Docker](https://www.docker.com/). Building boils down to:

```
docker build --tag openpgp-ca ./
```

This will build the image and tag it as `openpgp-ca`. Once built, you can run it as:

```
docker run openpgp-ca
```

You should see the help output. Running any `openpgp-ca` command is easy, just add it at the end, like so:

```
docker run openpgp-ca ca new example.org
```

However, since it's running in Docker, the database does not persist. The database is kept in `/var/run/openpgp-ca/` inside the container. Therefore, you might want to do a volume-mount:

```
docker run -v "/some/host/directory/:/var/run/openpgp-ca/" openpgp-ca ca new example.org
```

An example centralized workflow of creating a CA and a user would thus be:

```
docker run -v "/some/host/directory/:/var/run/openpgp-ca/" openpgp-ca ca new example.org
docker run -v "/some/host/directory/:/var/run/openpgp-ca/" openpgp-ca user add --email alice@example.org --email a@example.org --name Alicia
docker run -v "/some/host/directory/:/var/run/openpgp-ca/" openpgp-ca user add --email bob@example.org
docker run -v "/some/host/directory/:/var/run/openpgp-ca/" openpgp-ca user list
```

Obviously for regular use you might use more automated tools like [`docker-compose`](https://docs.docker.com/compose/).
