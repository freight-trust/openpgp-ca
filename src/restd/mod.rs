// Copyright 2019-2020 Heiko Schaefer <heiko@schaefer.name>
//
// This file is part of OpenPGP CA
// https://gitlab.com/openpgp-ca/openpgp-ca
//
// SPDX-FileCopyrightText: 2019-2020 Heiko Schaefer <heiko@schaefer.name>
// SPDX-License-Identifier: GPL-3.0-or-later

//! REST Interface for OpenPGP CA.
//! This is an experimental API for use at FSFE.

use std::ops::Deref;

use rocket::response::status::BadRequest;
use rocket_contrib::json::Json;

mod cli;
pub mod client;
pub mod oca_json;
mod util;

use std::collections::HashSet;

use openpgp::Cert;
use sequoia_openpgp as openpgp;

use super::ca::OpenpgpCa;
use super::models;
use oca_json::*;

use once_cell::sync::OnceCell;

static DB: OnceCell<Option<String>> = OnceCell::new();

thread_local! {
    static CA: OpenpgpCa = OpenpgpCa::new(DB.get().unwrap().as_deref())
        .expect("OpenPGP CA new() failed - database problem?");
}

// key size limit (armored): 1 mbyte
const KEY_SIZE_LIMIT: usize = 1024 * 1024;

// CA certifications are good for 365 days
const CERTIFICATION_DAYS: Option<u64> = Some(365);

fn check_and_normalize_cert(
    ca: &OpenpgpCa,
    certificate: &Certificate,
) -> std::result::Result<Cert, ReturnError> {
    let res = OpenpgpCa::armored_to_cert(&certificate.cert);

    if let Ok(mut cert) = res {
        // private keys are illegal
        if cert.is_tsk() {
            return Err(ReturnError::new(
                ReturnStatus::PrivateKey,
                String::from("The user provided private key material"),
            ));
        }

        // reject unreasonably big keys
        if certificate.cert.len() > KEY_SIZE_LIMIT {
            return Err(ReturnError::new(
                ReturnStatus::KeySizeLimit,
                format!(
                    "User certificate is too long ({} bytes)",
                    certificate.cert.len(),
                ),
            ));
        }

        // get the domain of this CA
        let my_domain = ca
            .get_ca_domain()
            .expect("Error while getting the CA's domain");

        // split up user_ids between "external" and "internal" emails, then:
        if let Ok((int_provided, _)) =
            util::split_emails(&my_domain, &certificate.email)
        {
            let mut int_remaining: HashSet<_> = int_provided.iter().collect();
            let mut filter_uid = Vec::new();

            for user_id in cert.userids() {
                if let Ok(email) = user_id.email() {
                    if let Some(email) = email {
                        let in_domain =
                            util::is_email_in_domain(&email, &my_domain)
                                .map_err(|_e| {
                                    ReturnError::new(
                                        ReturnStatus::BadEmail,
                                        format!(
                                            "Bad email address provided: '{}'",
                                            email
                                        ),
                                    )
                                })?;

                        if in_domain {
                            // a) all provided internal "email" entries must exist in cert user_ids
                            if int_remaining.contains(&email) {
                                int_remaining.remove(&email);
                            } else {
                                // b) flag additional "internal" emails for removal
                                filter_uid.push(user_id.userid().clone());
                            }
                        }
                    }
                } else {
                    return Err(ReturnError::new(
                        ReturnStatus::BadKey,
                        format!("Bad user_id '{:?}' in OpenPGP Key", user_id),
                    ));
                }
            }

            // b) strip additional "internal"s user_ids from the Cert
            for filter in filter_uid {
                cert = util::user_id_filter(&cert, &filter).map_err(|_e| {
                    ReturnError::new(
                        ReturnStatus::InternalError,
                        format!(
                            "Error while filtering user_id {:?} from Cert",
                            filter,
                        ),
                    )
                })?;
            }

            if !int_remaining.is_empty() {
                // some provided internal "email" entries do not exist in user_ids
                // -> not ok!
                return Err(ReturnError::new(
                    ReturnStatus::KeyMissingLocalUserId,
                    format!(
                        "User certificate does not contain user_ids for '{:?}'",
                        int_remaining,
                    ),
                ));
            }

            Ok(cert)
        } else {
            // FIXME: return more detailed information
            Err(ReturnError::new(
                ReturnStatus::BadEmail,
                String::from("Error with provided email addresses"),
            ))
        }
    } else {
        Err(ReturnError::new(
            ReturnStatus::BadKey,
            String::from(
                "Error parsing the user-provided armored OpenPGP key",
            ),
        ))
    }
}

fn load_certificate_data(
    ca: &OpenpgpCa,
    cert: &models::Cert,
) -> Result<Certificate, ReturnError> {
    let res = {
        let user =
            ca.cert_get_users(&cert)
                .map_err(|e| {
                    ReturnError::new(
                        ReturnStatus::InternalError,
                        format!(
                            "load_certificate_data: error while loading users \
                        '{}'", e
                        ),
                    )
                })?
                .unwrap();

        let emails = ca.emails_get(&cert).map_err(|e| {
            ReturnError::new(
                ReturnStatus::InternalError,
                format!(
                    "load_certificate_data: error while loading emails '{}'",
                    e
                ),
            )
        })?;

        let rev = ca.revocations_get(&cert).map_err(|e| {
            ReturnError::new(
                ReturnStatus::InternalError,
                format!(
                    "load_certificate_data: error while loading revocations\
                     '{}'",
                    e
                ),
            )
        })?;

        Certificate::from(&cert, &user, &emails, &rev)
    };

    Ok(res)
}

#[get("/certs/by_email/<email>")]
fn certs_by_email(
    email: String,
) -> Result<Json<Vec<ReturnJSON>>, BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let mut certificates = Vec::new();

        let certs = ca.certs_get(&email).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!(
                    "certs_by_email: error loading certificates data '{}'",
                    e
                ),
            )
        })?;

        for c in certs {
            let cert =
                OpenpgpCa::armored_to_cert(&c.pub_cert).map_err(|e| {
                    ReturnError::bad_req(
                        ReturnStatus::InternalError,
                        format!(
                            "certs_by_email: error during armored_to_cert '{}'",
                            e
                        ),
                    )
                })?;

            let certificate = load_certificate_data(&ca, &c)
                .map_err(|e| BadRequest(Some(Json(e))))?;

            let r = ReturnJSON {
                cert_info: (&cert).into(),
                action: None,
                certificate,
            };

            certificates.push(r);
        }

        Ok(Json(certificates))
    })
}

#[get("/certs/by_fp/<fp>")]
fn certs_by_fp(
    fp: String,
) -> Result<Json<Option<ReturnJSON>>, BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let c = ca.cert_get_by_fingerprint(&fp).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!("certs_by_fp: error loading certificate data '{}'", e),
            )
        })?;

        if let Some(c) = c {
            let cert =
                OpenpgpCa::armored_to_cert(&c.pub_cert).map_err(|e| {
                    ReturnError::bad_req(
                        ReturnStatus::InternalError,
                        format!(
                            "certs_by_fp: error during armored_to_cert '{}'",
                            e
                        ),
                    )
                })?;

            let certificate = load_certificate_data(&ca, &c)
                .map_err(|e| BadRequest(Some(Json(e))))?;

            Ok(Json(Some(ReturnJSON {
                cert_info: (&cert).into(),
                certificate,
                action: None,
            })))
        } else {
            Ok(Json(None))
        }
    })
}

/// Similar to "post_user", but doesn't commit data to DB.
///
/// Returns information about what the commit would result in.
#[get("/certs/check", data = "<certificate>", format = "json")]
fn check_cert(
    certificate: Json<Certificate>,
) -> Result<Json<ReturnJSON>, BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let certificate = certificate.into_inner();
        let res = check_and_normalize_cert(&ca, &certificate);

        // FIXME: do some more linting?

        if let Ok(cert) = res {
            // check if fingerprint exists in db
            // -> action "new" or "update"

            let fp = cert.fingerprint().to_hex();
            let res = ca.cert_get_by_fingerprint(&fp);
            if let Ok(cert_by_fp) = res {
                let action = if cert_by_fp.is_some() {
                    Action::Update
                } else {
                    Action::New
                };

                let armor = OpenpgpCa::cert_to_armored(&cert);
                if let Ok(key) = armor {
                    let mut certificate = certificate;
                    certificate.cert = key;

                    Ok(Json(ReturnJSON {
                        cert_info: (&cert).into(),
                        certificate,
                        action: Some(action),
                    }))
                } else {
                    Err(ReturnError::bad_req(
                        ReturnStatus::BadKey,
                        armor.err().unwrap().to_string(),
                    ))
                }
            } else {
                Err(ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    format!(
                        "Error during database lookup by fingerprint: {}",
                        res.err().unwrap().to_string(),
                    ),
                ))
            }
        } else {
            Err(BadRequest(Some(Json(res.err().unwrap()))))
        }
    })
}

/// Store new User-Key data in the OpenPGP CA database.
///
/// This function is intended for the following workflows:
///
/// 1) add an entirely new user
/// 2) store an update for an existing key (i.e. same fingerprint)
/// 2a) one notable specific case of this:
///     the user adds a revocation to their key (as an update).
/// 3) store a "new" (i.e. different fingerprint) key for the same user
#[post("/certs", data = "<certificate>", format = "json")]
fn post_user(
    certificate: Json<Certificate>,
) -> Result<(), BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let cert = certificate.into_inner();

        // check and normalize user-provided public key
        let cert_normalized = check_and_normalize_cert(ca, &cert)
            .map_err(|e| BadRequest(Some(Json(e))))?;

        // check if a cert with this fingerprint exists already
        let fp = cert_normalized.fingerprint().to_hex();

        let cert_by_fp = ca.cert_get_by_fingerprint(&fp).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!("Error during database lookup by fingerprint: {}", e,),
            )
        })?;

        if let Some(cert_by_fp) = cert_by_fp {
            // fingerprint of the key already exists
            //   -> merge data, update existing key
            let existing = OpenpgpCa::armored_to_cert(&cert_by_fp.pub_cert)
                .map_err(|_e| {
                    ReturnError::bad_req(
                        ReturnStatus::InternalError,
                        format!(
                            "Error while deserializing armored Cert: {}",
                            &cert_by_fp.pub_cert,
                        ),
                    )
                })?;

            let updated = existing.merge(cert_normalized).map_err(|_e| {
                ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    String::from("Error while merging Certs"),
                )
            })?;

            let armored =
                OpenpgpCa::cert_to_armored(&updated).map_err(|_e| {
                    ReturnError::bad_req(
                        ReturnStatus::InternalError,
                        String::from("Error while serializing merged Cert"),
                    )
                })?;

            ca.cert_import_update(&armored).map_err(|e| {
                ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    format!(
                        "Error updating Cert in database: {}",
                        e.to_string()
                    ),
                )
            })?;
        } else {
            // fingerprint doesn't exist yet -> new cert

            let armored = OpenpgpCa::cert_to_armored(&cert_normalized)
                .map_err(|_e| {
                    ReturnError::bad_req(
                        ReturnStatus::InternalError,
                        String::from("Error while serializing new Cert"),
                    )
                })?;

            ca.cert_import_new(
                &armored,
                vec![],
                cert.name.as_deref(),
                cert.email
                    .iter()
                    .map(|e| e.deref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                CERTIFICATION_DAYS,
            )
            .map_err(|e| {
                ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    format!(
                        "Error importing Cert into database: {}",
                        e.to_string()
                    ),
                )
            })?;
        }

        Ok(())
    })
}

/// Mark a certificate as "deactivated".
/// It will continue to be listed and exported to WKD.
/// However, the certification by our CA will expire and not get renewed.
///
/// This approach is probably appropriate in most cases to phase out a
/// certificate.
#[post("/certs/deactivate/<fp>")]
fn deactivate_cert(fp: String) -> Result<(), BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let cert = ca.cert_get_by_fingerprint(&fp).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!("Error looking up Fingerprint '{}'", e),
            )
        })?;

        if let Some(mut cert) = cert {
            cert.inactive = true;

            ca.cert_update(&cert).map_err(|e| {
                ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    format!("Error updating Cert '{}'", e),
                )
            })?;
        } else {
            return Err(ReturnError::bad_req(
                ReturnStatus::NotFound,
                format!("Fingerprint '{}' not found", fp),
            ));
        }

        Ok(())
    })
}

/// Remove a cert from the OpenPGP CA database, by fingerprint.
/// As a result, the cert will not be exported to WKD anymore.
///
/// Note: the CA certification will not get renewed in this case, so it will
/// expire.
///
/// CAUTION:
/// This method is probably rarely appropriate. In most cases, it's better
/// to "deactivate" a cert.
#[delete("/certs/<fp>")]
fn delist_cert(fp: String) -> Result<(), BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        let cert = ca.cert_get_by_fingerprint(&fp).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!("Error looking up Fingerprint '{}'", e),
            )
        })?;

        if let Some(mut cert) = cert {
            cert.delisted = true;

            ca.cert_update(&cert).map_err(|e| {
                ReturnError::bad_req(
                    ReturnStatus::InternalError,
                    format!("Error updating Cert '{}'", e),
                )
            })?;
        } else {
            return Err(ReturnError::bad_req(
                ReturnStatus::NotFound,
                format!("Fingerprint '{}' not found", fp),
            ));
        }

        Ok(())
    })
}

/// Refresh CA certifications on all user certs
///
/// For certifications which are going to expire soon:
/// Make a new certification, unless the user cert is marked as "deactivated".
#[post("/refresh_ca_certifications")]
fn refresh_certifications() -> Result<(), BadRequest<Json<ReturnError>>> {
    CA.with(|ca| {
        ca.certs_refresh_ca_certifications(30, 365).map_err(|e| {
            ReturnError::bad_req(
                ReturnStatus::InternalError,
                format!(
                    "Error during certs_refresh_ca_certifications '{}'",
                    e
                ),
            )
        })?;

        Ok(())
    })
}

/// Poll for updates to user keys (e.g. on https://keys.openpgp.org/)
#[post("/poll_updates")]
fn poll_for_updates() -> String {
    unimplemented!()
}

pub fn run(db: Option<String>) -> rocket::Rocket {
    DB.set(db).unwrap();

    rocket::ignite().mount(
        "/",
        routes![
            certs_by_email,
            certs_by_fp,
            check_cert,
            post_user,
            deactivate_cert,
            delist_cert,
            refresh_certifications,
            poll_for_updates,
        ],
    )
}
