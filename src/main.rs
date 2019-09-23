// Copyright 2019 Heiko Schaefer heiko@schaefer.name
//
// This file is part of OpenPGP CA.
//
// OpenPGP CA is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// OpenPGP CA is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with OpenPGP CA.  If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate clap;
extern crate failure;
extern crate sequoia_openpgp as openpgp;

use std::process::exit;

use clap::App;

pub mod ca;
pub mod models;
pub mod schema;
pub mod db;
pub mod pgp;

pub type Result<T> = ::std::result::Result<T, failure::Error>;

fn real_main() -> Result<()> {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let db = matches.value_of("database");

    let mut ca = ca::Ca::new(db);

    match matches.subcommand() {
        ("init", Some(_m)) => {
            ca.init();
        }
        ("ca", Some(m)) => {
            match m.subcommand() {
                ("new", Some(m2)) => {
                    match (m2.value_of("name"), m2.values_of("email")) {
                        (Some(name), Some(email)) => {
                            let emails = email.into_iter().collect::<Vec<_>>();
                            ca.ca_new(name, &emails)?;
                        }
                        _ => unimplemented!(),
                    }
                }
                ("delete", Some(m2)) => {
                    match m2.value_of("name") {
                        Some(name) => ca.ca_delete(name)?,
                        _ => unimplemented!(),
                    }
                }
                ("list", Some(_m2)) => {
                    ca.list_cas();
                }

                _ => unimplemented!(),
            }
        }
        ("user", Some(m)) => {
            match m.subcommand() {
                ("add", Some(m2)) => {
                    match m2.values_of("email") {
                        Some(email) => {
                            let email_vec = email.into_iter()
                                .collect::<Vec<_>>();

                            let name = m2.value_of("name");

                            // TODO
                            // .arg(Arg::with_name("key_profile")
                            //  .long("key_profile")
                            //  .value_name("key_profile")
                            //  .help("Key Profile"))

                            let ca_name = m2.value_of("ca_name").unwrap();


                            ca.user_new(name, Some(email_vec.as_ref()),
                                        ca_name)?;
                        }
                        _ => unimplemented!(),
                    }
                }
                ("import", Some(m2)) => {
                    match m2.values_of("email") {
                        Some(email) => {
                            let email_vec = email.into_iter()
                                .collect::<Vec<_>>();

                            let name = m2.value_of("name");

                            let key_file = m2.value_of("key_file").unwrap();
                            let revocation_file = m2.value_of("revocation_file");

                            let ca_name = m2.value_of("ca_name").unwrap();

                            ca.user_import(name, Some(email_vec.as_ref()),
                                           ca_name, key_file, revocation_file,
                            )?;
                        }
                        _ => unimplemented!(),
                    }
                }
                ("list", Some(_m2)) => {
                    ca.list_users()?;
                }

                _ => unimplemented!(),
            }
        }
        ("bridge", Some(m)) => {
            match m.subcommand() {
                ("new", Some(m2)) => {
                    match m2.values_of("regex") {
                        Some(regex) => {
                            let regex_vec = regex.into_iter()
                                .collect::<Vec<_>>();

                            let key_file =
                                m2.value_of("remote_key_file").unwrap();

                            let name = m2.value_of("name").unwrap();

                            let ca_name = m2.value_of("ca_name").unwrap();

                            ca.bridge_new(name, ca_name, key_file,
                                          Some(regex_vec.as_ref()))?;
                        }
                        _ => unimplemented!(),
                    }
                }
                ("revoke", Some(m2)) => {
                    let name = m2.value_of("name").unwrap();

                    ca.bridge_revoke(name)?;
                }
                ("list", Some(_m2)) => {
                    ca.list_bridges()?;
                }

                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }

    Ok(())
}

// -----------------

fn main() {
    if let Err(e) = real_main() {
        let mut cause = e.as_fail();
        eprint!("{}", cause);
        while let Some(c) = cause.cause() {
            eprint!(":\n  {}", c);
            cause = c;
        }
        eprintln!();
        exit(2);
    }
}