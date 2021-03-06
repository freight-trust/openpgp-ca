<!--
SPDX-FileCopyrightText: 2019-2020 Heiko Schaefer <heiko@schaefer.name>
SPDX-License-Identifier: GPL-3.0-or-later
-->

Once our OpenPGP CA database is populated with data, we may want to
inspect that data.

In this chapter we assume that our OpenPGP CA instance contains the two users
Alice and Bob.

### Listing all users

We can inspect the state of all users in our OpenPGP CA instance like this:

`$ openpgp-ca -d example.oca user list`

```
OpenPGP key for 'Alice Adams'
 fingerprint F27CB2E92C3E01DA1C656FB21758251C75E25DDD
 user cert (or subkey) signed by CA: true
 user cert has tsigned CA: true
 - email alice@example.org
 no expiration date is set for this user key
 1 revocation certificate(s) available

OpenPGP key for 'Bob Baker'
 fingerprint 0EE935F56AC4381E007370E956A10EB1ABED2321
 user cert (or subkey) signed by CA: true
 user cert has tsigned CA: true
 - email bob@example.org
 expires: 12/08/2020
 1 revocation certificate(s) available
```

### Exporting keys

Export an individual user key (the public key is
printed to stdout):

`$ openpgp-ca -d example.oca user export -e alice@example.org`

```
-----BEGIN PGP PUBLIC KEY BLOCK-----
Comment: F27C B2E9 2C3E 01DA 1C65  6FB2 1758 251C 75E2 5DDD
Comment: Alice Adams <alice@example.org>

xjMEXv8FyxYJKwYBBAHaRw8BAQdADeFuwt/+AtkUWNMxmi/nKwpF/Nnf76QX7qNi
v2JWUxjCfgQfFgoADwWCXv8FywIVCgKbAQIeAQAhCRAXWCUcdeJd3RYhBPJ8suks
[...]
6Sw+AdocZW+yF1glHHXiXd1lcgD/byHHRjsKEux07gYeGUs+MpP4trLr6SL3Gyqf
bRcVqcMA/0RsK9WcWw5ZHmVqCM7OXOu1Fdk81xqVJVggKhdgMwcD
=TFLi
-----END PGP PUBLIC KEY BLOCK-----
```

To output all public user keys from OpenPGP CA to stdout:

`$ openpgp-ca -d example.oca user export`

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

xjMEXv8FyxYJKwYBBAHaRw8BAQdADeFuwt/+AtkUWNMxmi/nKwpF/Nnf76QX7qNi
v2JWUxjCfgQfFgoADwWCXv8FywIVCgKbAQIeAQAhCRAXWCUcdeJd3RYhBPJ8suks
[...]
L41nIJEWza8ZhWPINdDFWAnOEWuDF/312/k3mZBs4IGCN0NjFKMQKL2dBTacWzZz
8J0nPe2QqePJHVH4
=F05k
-----END PGP PUBLIC KEY BLOCK-----
```

To output the public key of our OpenPGP CA instance:

`$ openpgp-ca -d example.oca ca export`

```
-----BEGIN PGP PUBLIC KEY BLOCK-----
Comment: 138C 1D33 E462 4BFB CCC4  0C20 3EA1 01D6 8A4B 92F5
Comment: OpenPGP CA <openpgp-ca@example.org>

xjMEXv8FxxYJKwYBBAHaRw8BAQdAPULzjk6Hr+0PahT42WxfaDSgHfqPOmNLB4q9
fVC1g9jCfgQfFgoADwWCXv8FxwIVCgKbAQIeAQAhCRA+oQHWikuS9RYhBBOMHTPk
[...]
IvO+f3pFqLZEzoFJXUm4oxr7CXADfWUgQj7yAtIa3ZUA/ApZKKmp0E/S8VGjhe0Q
Ni+wbKBJIe94AE3A6ZggKd4B
=vt2K
-----END PGP PUBLIC KEY BLOCK-----
```

### Checking certifications

To check if all keys are mutually certified:

- All user keys have tsigned the CA key, and
- the CA key has certified all user keys.
 
`$ openpgp-ca -d example.oca user check certifications`

```
Checked 2 user keys, 2 of them had good certifications in both directions.
```

### Checking expiry of user keys
 
To get an overview of the expiry of user keys:
 
`$ openpgp-ca -d example.oca user check expiry`

```
name Alice Adams, fingerprint F27CB2E92C3E01DA1C656FB21758251C75E25DDD
 no expiration date is set for this user key

name Bob Baker, fingerprint 0EE935F56AC4381E007370E956A10EB1ABED2321
 expires: 12/08/2020
```

To check if any user keys will expire within a specified number of days:
 
`$ openpgp-ca -d example.oca user check expiry --days 60`

```
name Alice Adams, fingerprint F27CB2E92C3E01DA1C656FB21758251C75E25DDD
 no expiration date is set for this user key

name Bob Baker, fingerprint 0EE935F56AC4381E007370E956A10EB1ABED2321
 expires: 12/08/2020
 user cert EXPIRED/EXPIRING!
```

This output shows us that Bob's key will have expired 60 days from now.
