// Copyright 2019-2020 Heiko Schaefer <heiko@schaefer.name>
//
// This file is part of OpenPGP CA
// https://gitlab.com/openpgp-ca/openpgp-ca
//
// SPDX-FileCopyrightText: 2019-2020 Heiko Schaefer <heiko@schaefer.name>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Very basic client wrapper for the OpenPGP CA restd,
//! intended for use in integration tests.

use reqwest::{Response, StatusCode};

use crate::restd::oca_json::{Certificate, ReturnError, ReturnJSON};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

pub struct Client {
    client: reqwest::Client,
    uri: String,
}

impl Client {
    pub fn new<S: Into<String>>(uri: S) -> Self {
        Client {
            client: reqwest::Client::new(),
            uri: uri.into(),
        }
    }

    async fn map_result(
        resp: Result<Response, reqwest::Error>,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        match resp {
            Ok(o) => match o.status() {
                StatusCode::OK => {
                    if o.content_length() == Some(0) {
                        Ok(None)
                    } else {
                        let resp =
                            o.json::<Option<ReturnJSON>>().await.unwrap();

                        Ok(resp)
                    }
                }
                StatusCode::BAD_REQUEST => {
                    let resp = o.json::<ReturnError>().await.unwrap();

                    Err(resp)
                }
                _ => panic!("unexpected status code {}", o.status()),
            },
            Err(e) => {
                panic!("error {}", e);
            }
        }
    }

    async fn map_result_vec(
        resp: Result<Response, reqwest::Error>,
    ) -> Result<Vec<ReturnJSON>, ReturnError> {
        match resp {
            Ok(o) => match o.status() {
                StatusCode::OK => {
                    let resp = o.json::<Vec<ReturnJSON>>().await.unwrap();

                    Ok(resp)
                }
                StatusCode::BAD_REQUEST => {
                    let resp = o.json::<ReturnError>().await.unwrap();

                    Err(resp)
                }
                _ => panic!("unexpected status code {}", o.status()),
            },
            Err(e) => {
                panic!("error {}", e);
            }
        }
    }

    pub async fn check(
        &self,
        cert: &Certificate,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        let cert_json = serde_json::to_string(&cert).unwrap();

        let resp = self
            .client
            .get(&format!("{}certs/check", &self.uri))
            .body(cert_json)
            .send()
            .await;

        Client::map_result(resp).await
    }

    pub async fn persist(
        &self,
        cert: &Certificate,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        let cert_json = serde_json::to_string(&cert).unwrap();

        let mut header_map = HeaderMap::new();
        header_map.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=UTF-8"),
        );

        let resp = self
            .client
            .post(&format!("{}certs", &self.uri))
            .headers(header_map)
            .body(cert_json)
            .send()
            .await;

        Client::map_result(resp).await
    }

    pub async fn get_by_email(
        &self,
        email: String,
    ) -> Result<Vec<ReturnJSON>, ReturnError> {
        let resp = self
            .client
            .get(&format!("{}certs/by_email/{}", &self.uri, email))
            .send()
            .await;

        Client::map_result_vec(resp).await
    }

    pub async fn get_by_fp(
        &self,
        fp: String,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        let resp = self
            .client
            .get(&format!("{}certs/by_fp/{}", &self.uri, fp))
            .send()
            .await;

        Client::map_result(resp).await
    }

    pub async fn deactivate(
        &self,
        fp: String,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        let resp = self
            .client
            .post(&format!("{}certs/deactivate/{}", &self.uri, fp))
            .send()
            .await;

        Client::map_result(resp).await
    }

    pub async fn delist(
        &self,
        fp: String,
    ) -> Result<Option<ReturnJSON>, ReturnError> {
        let resp = self
            .client
            .delete(&format!("{}certs/{}", &self.uri, fp))
            .send()
            .await;

        Client::map_result(resp).await
    }
}
