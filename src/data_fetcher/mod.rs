mod common;

use common::{BaseHttpClient, Form, Headers, Query};
use reqwest::Error as ReqwestError;

use std::convert::TryInto;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DataFetcher {
    client: reqwest::Client,
}

impl Default for DataFetcher {
    fn default() -> Self {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .build()
            // building with these options cannot fail
            .unwrap();
        Self { client }
    }
}

impl DataFetcher {
    async fn request<D>(
        &self,
        method: Method,
        url: &str,
        headers: Option<&Headers>,
        add_data: D,
    ) -> Result<String, ReqwestError>
    where
        D: Fn(RequestBuilder) -> RequestBuilder,
    {
        let mut request = self.client.request(method.clone(), url);

        if let Some(headers) = headers {
            let headers = headers.try_into().unwrap();

            request = request.headers(headers);
        }

        request = add_data(request);

        log::info!("Making request {:?}", request);
        let response = request.send().await?;

        match response.error_for_status_ref() {
            Ok(_) => Ok(response.text().await?),
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl BaseHttpClient for DataFetcher {
    type Error = ReqwestError;

    #[inline]
    async fn get(
        &self,
        url: &str,
        headers: Option<&Headers>,
        payload: &Query,
    ) -> Result<String, Self::Error> {
        self.request(Method::GET, url, headers, |req| req.query(payload))
            .await
    }

    #[inline]
    async fn post(
        &self,
        url: &str,
        headers: Option<&Headers>,
        payload: &Value,
    ) -> Result<String, Self::Error> {
        self.request(Method::POST, url, headers, |req| req.json(payload))
            .await
    }

    #[inline]
    async fn post_form(
        &self,
        url: &str,
        headers: Option<&Headers>,
        payload: &Form<'_>,
    ) -> Result<String, Self::Error> {
        self.request(Method::POST, url, headers, |req| req.form(payload))
            .await
    }

    #[inline]
    async fn put(
        &self,
        url: &str,
        headers: Option<&Headers>,
        payload: &Value,
    ) -> Result<String, Self::Error> {
        self.request(Method::PUT, url, headers, |req| req.json(payload))
            .await
    }

    #[inline]
    async fn delete(
        &self,
        url: &str,
        headers: Option<&Headers>,
        payload: &Value,
    ) -> Result<String, Self::Error> {
        self.request(Method::DELETE, url, headers, |req| req.json(payload))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get() {
        let mock_server = MockServer::start().await;
        let test_url = mock_server.uri();
        let response_body = json!({"status": "success"}).to_string();

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body.clone()))
            .mount(&mock_server)
            .await;

        let client = DataFetcher::default();
        let result = client
            .get(&(test_url + "/test"), None, &Query::default())
            .await
            .unwrap();

        assert_eq!(result, response_body);
    }

    #[tokio::test]
    async fn test_post() {
        let mock_server = MockServer::start().await;
        let test_url = mock_server.uri();
        let response_body = json!({"status": "received"}).to_string();

        Mock::given(method("POST"))
            .and(path("/test"))
            .and(wiremock::matchers::body_json(json!({"data": "example"})))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body.clone()))
            .mount(&mock_server)
            .await;

        let client = DataFetcher::default();
        let payload = json!({"data": "example"});
        let result = client
            .post(&(test_url + "/test"), None, &payload)
            .await
            .unwrap();

        assert_eq!(result, response_body);
    }
}
