#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::missing_panics_doc,
    clippy::unwrap_used
)]
use std::{
    collections::{HashMap, HashSet},
    env,
    io::Read,
    path::MAIN_SEPARATOR,
    time::Duration,
};

use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use test_utils::connector_auth;
use thirtyfour::{components::SelectElement, prelude::*, WebDriver};

#[derive(Clone)]
pub enum Event<'a> {
    RunIf(Assert<'a>, Vec<Event<'a>>),
    EitherOr(Assert<'a>, Vec<Event<'a>>, Vec<Event<'a>>),
    Assert(Assert<'a>),
    Trigger(Trigger<'a>),
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum Trigger<'a> {
    Goto(&'a str),
    Click(By),
    ClickNth(By, usize),
    SelectOption(By, &'a str),
    ChangeQueryParam(&'a str, &'a str),
    SwitchTab(Position),
    SwitchFrame(By),
    Find(By),
    Query(By),
    SendKeys(By, &'a str),
    Sleep(u64),
}

#[derive(Clone)]
pub enum Position {
    Prev,
    Next,
}
#[derive(Clone)]
pub enum Selector {
    Title,
    QueryParamStr,
}

#[derive(Clone)]
pub enum Assert<'a> {
    Eq(Selector, &'a str),
    Contains(Selector, &'a str),
    ContainsAny(Selector, Vec<&'a str>),
    EitherOfThemExist(&'a str, &'a str),
    IsPresent(&'a str),
    IsElePresent(By),
    IsPresentNow(&'a str),
}

pub static CHEKOUT_BASE_URL: &str = "https://hs-payments-test.netlify.app";
#[async_trait]
pub trait SeleniumTest {
        /// Retrieves the saved test cases as a JSON value.
    fn get_saved_testcases(&self) -> serde_json::Value {
        get_saved_testcases()
    }
        /// This method returns the ConnectorAuthentication configuration for the current instance.
    fn get_configs(&self) -> connector_auth::ConnectorAuthentication {
        get_configs()
    }
        /// Retries clicking on a web element a specified number of times at a given interval
    ///
    /// # Arguments
    /// * `times` - The number of times to retry clicking the element
    /// * `interval` - The interval in seconds between each retry
    /// * `driver` - The WebDriver used for interacting with the web page
    /// * `by` - The method used to locate the web element (e.g. by ID, by class name)
    ///
    /// # Returns
    /// * `Result<(), WebDriverError>` - A result indicating success or an error if clicking the element failed
    ///
    async fn retry_click(
        &self,
        times: i32,
        interval: u64,
        driver: &WebDriver,
        by: By,
    ) -> Result<(), WebDriverError> {
        let mut res = Ok(());
        for _i in 0..times {
            res = self.click_element(driver, by.clone()).await;
            if res.is_err() {
                tokio::time::sleep(Duration::from_secs(interval)).await;
            } else {
                break;
            }
        }
        return res;
    }
    fn get_connector_name(&self) -> String;
        /// This method iterates through a vector of Event enums and performs corresponding actions based on the event type. It uses a WebDriver to execute various actions such as asserting, running conditional events, triggering specific actions like clicking, sending keys, changing query parameters, and more. It returns a Result indicating whether the actions were completed successfully or if a WebDriverError occurred.
    async fn complete_actions(
        &self,
        driver: &WebDriver,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        for action in actions {
            match action {
                Event::Assert(assert) => match assert {
                    Assert::Contains(selector, text) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            assert!(url.query().unwrap().contains(text))
                        }
                        _ => assert!(driver.title().await?.contains(text)),
                    },
                    Assert::ContainsAny(selector, search_keys) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            assert!(search_keys
                                .iter()
                                .any(|key| url.query().unwrap().contains(key)))
                        }
                        _ => assert!(driver.title().await?.contains(search_keys.first().unwrap())),
                    },
                    Assert::EitherOfThemExist(text_1, text_2) => assert!(
                        is_text_present_now(driver, text_1).await?
                            || is_text_present_now(driver, text_2).await?
                    ),
                    Assert::Eq(_selector, text) => assert_eq!(driver.title().await?, text),
                    Assert::IsPresent(text) => {
                        assert!(is_text_present(driver, text).await?)
                    }
                    Assert::IsElePresent(selector) => {
                        assert!(is_element_present(driver, selector).await?)
                    }
                    Assert::IsPresentNow(text) => {
                        assert!(is_text_present_now(driver, text).await?)
                    }
                },
                Event::RunIf(con_event, events) => match con_event {
                    Assert::Contains(selector, text) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            if url.query().unwrap().contains(text) {
                                self.complete_actions(driver, events).await?;
                            }
                        }
                        _ => assert!(driver.title().await?.contains(text)),
                    },
                    Assert::ContainsAny(selector, keys) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            if keys.iter().any(|key| url.query().unwrap().contains(key)) {
                                self.complete_actions(driver, events).await?;
                            }
                        }
                        _ => assert!(driver.title().await?.contains(keys.first().unwrap())),
                    },
                    Assert::Eq(_selector, text) => {
                        if text == driver.title().await? {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                    Assert::EitherOfThemExist(text_1, text_2) => {
                        if is_text_present_now(driver, text_1).await.is_ok()
                            || is_text_present_now(driver, text_2).await.is_ok()
                        {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                    Assert::IsPresent(text) => {
                        if is_text_present(driver, text).await.is_ok() {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                    Assert::IsElePresent(text) => {
                        if is_element_present(driver, text).await.is_ok() {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                    Assert::IsPresentNow(text) => {
                        if is_text_present_now(driver, text).await.is_ok() {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                },
                Event::EitherOr(con_event, success, failure) => match con_event {
                    Assert::Contains(selector, text) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            self.complete_actions(
                                driver,
                                if url.query().unwrap().contains(text) {
                                    success
                                } else {
                                    failure
                                },
                            )
                            .await?;
                        }
                        _ => assert!(driver.title().await?.contains(text)),
                    },
                    Assert::ContainsAny(selector, keys) => match selector {
                        Selector::QueryParamStr => {
                            let url = driver.current_url().await?;
                            self.complete_actions(
                                driver,
                                if keys.iter().any(|key| url.query().unwrap().contains(key)) {
                                    success
                                } else {
                                    failure
                                },
                            )
                            .await?;
                        }
                        _ => assert!(driver.title().await?.contains(keys.first().unwrap())),
                    },
                    Assert::Eq(_selector, text) => {
                        self.complete_actions(
                            driver,
                            if text == driver.title().await? {
                                success
                            } else {
                                failure
                            },
                        )
                        .await?;
                    }
                    Assert::EitherOfThemExist(text_1, text_2) => {
                        self.complete_actions(
                            driver,
                            if is_text_present_now(driver, text_1).await.is_ok()
                                || is_text_present_now(driver, text_2).await.is_ok()
                            {
                                success
                            } else {
                                failure
                            },
                        )
                        .await?;
                    }
                    Assert::IsPresent(text) => {
                        self.complete_actions(
                            driver,
                            if is_text_present(driver, text).await.is_ok() {
                                success
                            } else {
                                failure
                            },
                        )
                        .await?;
                    }
                    Assert::IsElePresent(by) => {
                        self.complete_actions(
                            driver,
                            if is_element_present(driver, by).await.is_ok() {
                                success
                            } else {
                                failure
                            },
                        )
                        .await?;
                    }
                    Assert::IsPresentNow(text) => {
                        self.complete_actions(
                            driver,
                            if is_text_present_now(driver, text).await.is_ok() {
                                success
                            } else {
                                failure
                            },
                        )
                        .await?;
                    }
                },
                Event::Trigger(trigger) => match trigger {
                    Trigger::Goto(url) => {
                        let saved_tests =
                            serde_json::to_string(&self.get_saved_testcases()).unwrap();
                        let conf = serde_json::to_string(&self.get_configs()).unwrap();
                        let configs = self.get_configs().automation_configs.unwrap();
                        let hs_base_url = configs
                            .hs_base_url
                            .unwrap_or_else(|| "http://localhost:8080".to_string());
                        let configs_url = configs.configs_url.unwrap();
                        let hs_api_keys = configs.hs_api_keys.unwrap();
                        let test_env = configs.hs_test_env.unwrap();
                        let script = &[
                            format!("localStorage.configs='{configs_url}'").as_str(),
                            format!("localStorage.current_env='{test_env}'").as_str(),
                            "localStorage.hs_api_key=''",
                            format!("localStorage.hs_api_keys='{hs_api_keys}'").as_str(),
                            format!("localStorage.base_url='{hs_base_url}'").as_str(),
                            format!("localStorage.hs_api_configs='{conf}'").as_str(),
                            format!("localStorage.saved_payments=JSON.stringify({saved_tests})")
                                .as_str(),
                            "localStorage.force_sync='true'",
                            format!(
                                "localStorage.current_connector=\"{}\";",
                                self.get_connector_name().clone()
                            )
                            .as_str(),
                        ]
                        .join(";");
                        driver.goto(url).await?;
                        driver.execute(script, Vec::new()).await?;
                    }
                    Trigger::Click(by) => {
                        self.retry_click(3, 5, driver, by.clone()).await?;
                    }
                    Trigger::ClickNth(by, n) => {
                        let ele = driver.query(by).all().await?.into_iter().nth(n).unwrap();
                        ele.wait_until().enabled().await?;
                        ele.wait_until().displayed().await?;
                        ele.wait_until().clickable().await?;
                        ele.scroll_into_view().await?;
                        ele.click().await?;
                    }
                    Trigger::Find(by) => {
                        driver.find(by).await?;
                    }
                    Trigger::Query(by) => {
                        driver.query(by).first().await?;
                    }
                    Trigger::SendKeys(by, input) => {
                        let ele = driver.query(by).first().await?;
                        ele.wait_until().displayed().await?;
                        ele.send_keys(&input).await?;
                    }
                    Trigger::SelectOption(by, input) => {
                        let ele = driver.query(by).first().await?;
                        let select_element = SelectElement::new(&ele).await?;
                        select_element.select_by_partial_text(input).await?;
                    }
                    Trigger::ChangeQueryParam(param, value) => {
                        let mut url = driver.current_url().await?;
                        let mut hash_query: HashMap<String, String> =
                            url.query_pairs().into_owned().collect();
                        hash_query.insert(param.to_string(), value.to_string());
                        let url_str = serde_urlencoded::to_string(hash_query)
                            .expect("Query Param update failed");
                        url.set_query(Some(&url_str));
                        driver.goto(url.as_str()).await?;
                    }
                    Trigger::Sleep(seconds) => {
                        tokio::time::sleep(Duration::from_secs(seconds)).await;
                    }
                    Trigger::SwitchTab(position) => match position {
                        Position::Next => {
                            let windows = driver.windows().await?;
                            if let Some(window) = windows.iter().next_back() {
                                driver.switch_to_window(window.to_owned()).await?;
                            }
                        }
                        Position::Prev => {
                            let windows = driver.windows().await?;
                            if let Some(window) = windows.into_iter().next() {
                                driver.switch_to_window(window.to_owned()).await?;
                            }
                        }
                    },
                    Trigger::SwitchFrame(by) => {
                        let iframe = driver.query(by).first().await?;
                        iframe.wait_until().displayed().await?;
                        iframe.clone().enter_frame().await?;
                    }
                },
            }
        }
        Ok(())
    }

        /// Asynchronously clicks on the specified web element using the provided WebDriver and locator strategy.
    ///
    /// # Arguments
    ///
    /// * `driver` - The WebDriver instance to use for interacting with the web element.
    /// * `by` - The locator strategy to use for finding the web element.
    ///
    /// # Returns
    ///
    /// Returns a Result indicating success or an error of type WebDriverError.
    ///
    async fn click_element(&self, driver: &WebDriver, by: By) -> Result<(), WebDriverError> {
        let ele = driver.query(by).first().await?;
        ele.wait_until().enabled().await?;
        ele.wait_until().displayed().await?;
        ele.wait_until().clickable().await?;
        ele.scroll_into_view().await?;
        ele.click().await
    }


        /// Asynchronously makes a redirection payment using the provided web driver and list of actions. This method supports failure retries by executing the steps again if the initial execution fails.
    async fn make_redirection_payment(
        &self,
        web_driver: WebDriver,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        // To support failure retries
        let result = self
            .execute_steps(web_driver.clone(), actions.clone())
            .await;
        if result.is_err() {
            self.execute_steps(web_driver, actions).await
        } else {
            result
        }
    }
        /// Executes a series of actions using the provided web driver, based on the automation configurations.
    ///
    /// If the automation configuration specifies to run a minimum number of steps, only the first 3 actions will be executed. Otherwise, all actions will be executed.
    ///
    /// # Arguments
    ///
    /// * `web_driver` - The web driver to use for executing the actions.
    /// * `actions` - A vector of Event containing the actions to be executed.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a WebDriverError if an error occurs during execution.
    ///
    async fn execute_steps(
        &self,
        web_driver: WebDriver,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        let config = self.get_configs().automation_configs.unwrap();
        if config.run_minimum_steps.unwrap() {
            self.complete_actions(&web_driver, actions.get(..3).unwrap().to_vec())
                .await
        } else {
            self.complete_actions(&web_driver, actions).await
        }
    }

        /// Asynchronously makes a Google Pay (GPay) payment using the provided WebDriver, URL, and list of actions.
    /// 
    /// # Arguments
    /// 
    /// * `web_driver` - The WebDriver to use for making the payment.
    /// * `url` - The URL of the payment page.
    /// * `actions` - A vector of events representing the actions to be performed for the payment.
    /// 
    /// # Returns
    /// 
    /// Returns a Result indicating success or failure, with the potential error being a WebDriverError.
    /// 
    async fn make_gpay_payment(
        &self,
        web_driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.execute_gpay_steps(web_driver, url, actions).await
    }
    /// Executes a series of Google Pay automation steps using the provided WebDriver and URL. The method triggers a sequence of events such as navigating to the specified URL, clicking on a Google Pay button, logging in with a Gmail account, switching tabs, waiting for elements to load, and more. The method then completes the default actions followed by the custom actions provided as a vector of Event enum. 
    async fn execute_gpay_steps(
        &self,
        web_driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        let config = self.get_configs().automation_configs.unwrap();
        let (email, pass) = (&config.gmail_email.unwrap(), &config.gmail_pass.unwrap());
        let default_actions = vec![
            Event::Trigger(Trigger::Goto(url)),
            Event::Trigger(Trigger::Click(By::Css(".gpay-button"))),
            Event::Trigger(Trigger::SwitchTab(Position::Next)),
            Event::Trigger(Trigger::Sleep(5)),
            Event::RunIf(
                Assert::EitherOfThemExist("Use your Google Account", "Sign in"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("identifierId"), email)),
                    Event::Trigger(Trigger::ClickNth(By::Tag("button"), 2)),
                    Event::EitherOr(
                        Assert::IsPresent("Welcome"),
                        vec![
                            Event::Trigger(Trigger::SendKeys(By::Name("Passwd"), pass)),
                            Event::Trigger(Trigger::Sleep(2)),
                            Event::Trigger(Trigger::Click(By::Id("passwordNext"))),
                            Event::Trigger(Trigger::Sleep(10)),
                        ],
                        vec![
                            Event::Trigger(Trigger::SendKeys(By::Id("identifierId"), email)),
                            Event::Trigger(Trigger::ClickNth(By::Tag("button"), 2)),
                            Event::Trigger(Trigger::SendKeys(By::Name("Passwd"), pass)),
                            Event::Trigger(Trigger::Sleep(2)),
                            Event::Trigger(Trigger::Click(By::Id("passwordNext"))),
                            Event::Trigger(Trigger::Sleep(10)),
                        ],
                    ),
                ],
            ),
            Event::Trigger(Trigger::SwitchFrame(By::Css(
                ".bootstrapperIframeContainerElement iframe",
            ))),
            Event::Assert(Assert::IsPresent("Gpay Tester")),
            Event::Trigger(Trigger::Click(By::ClassName("jfk-button-action"))),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
        ];
        self.complete_actions(&web_driver, default_actions).await?;
        self.complete_actions(&web_driver, actions).await
    }
        /// Makes an Affirm payment by completing a series of actions on the provided WebDriver.
    /// 
    /// # Arguments
    /// 
    /// * `driver` - The WebDriver used to interact with the web page.
    /// * `url` - The URL of the page where the payment process is initiated.
    /// * `actions` - Additional actions to be performed after the standard Affirm payment process.
    /// 
    /// # Returns
    /// 
    /// A Result indicating success or an error of type WebDriverError.
    /// 
    async fn make_affirm_payment(
        &self,
        driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(
            &driver,
            vec![
                Event::Trigger(Trigger::Goto(url)),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            ],
        )
        .await?;
        let mut affirm_actions = vec![
            Event::RunIf(
                Assert::IsPresent("Big purchase? No problem."),
                vec![
                    Event::Trigger(Trigger::SendKeys(
                        By::Css("input[data-testid='phone-number-field']"),
                        "(833) 549-5574", // any test phone number accepted by affirm
                    )),
                    Event::Trigger(Trigger::Click(By::Css(
                        "button[data-testid='submit-button']",
                    ))),
                    Event::Trigger(Trigger::SendKeys(
                        By::Css("input[data-testid='phone-pin-field']"),
                        "1234",
                    )),
                ],
            ),
            Event::Trigger(Trigger::Click(By::Css(
                "button[data-testid='skip-payment-button']",
            ))),
            Event::Trigger(Trigger::Click(By::Css("div[data-testid='indicator']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[data-testid='submit-button']",
            ))),
            Event::Trigger(Trigger::Click(By::Css("div[data-testid='indicator']"))),
            Event::Trigger(Trigger::Click(By::Css(
                "div[data-testid='disclosure-checkbox-indicator']",
            ))),
            Event::Trigger(Trigger::Click(By::Css(
                "button[data-testid='submit-button']",
            ))),
        ];
        affirm_actions.extend(actions);
        self.complete_actions(&driver, affirm_actions).await
    }
        /// This method is used to perform a series of actions to test a webhook by simulating a payment process, retrieving events from the outgoing webhook endpoint, and comparing the webhook response with the expected payment ID and status. If the comparison is successful, the method returns Ok(()), otherwise it retries a specified number of times before returning an error indicating that the webhook was not found.
    async fn make_webhook_test(
        &self,
        web_driver: WebDriver,
        payment_url: &str,
        actions: Vec<Event<'_>>,
        webhook_retry_time: u64,
        webhook_status: &str,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(
            &web_driver,
            vec![Event::Trigger(Trigger::Goto(payment_url))],
        )
        .await?;
        self.complete_actions(&web_driver, actions).await?; //additional actions needs to make a payment
        self.complete_actions(
            &web_driver,
            vec![Event::Trigger(Trigger::Goto(&format!(
                "{CHEKOUT_BASE_URL}/events"
            )))],
        )
        .await?;
        let element = web_driver.query(By::Css("h2.last-payment")).first().await?;
        let payment_id = element.text().await?;
        let retries = 3; // no of retry times
        for _i in 0..retries {
            let configs = self.get_configs().automation_configs.unwrap();
            let outgoing_webhook_url = configs.hs_webhook_url.unwrap().to_string();
            let client = reqwest::Client::new();
            let response = client.get(outgoing_webhook_url).send().await.unwrap(); // get events from outgoing webhook endpoint
            let body_text = response.text().await.unwrap();
            let data: WebhookResponse = serde_json::from_str(&body_text).unwrap();
            let last_three_events = data.data.get(data.data.len().saturating_sub(3)..).unwrap(); // Get the last three elements if available
            for last_event in last_three_events {
                let last_event_body = &last_event.step.request.body;
                let decoded_bytes = base64::engine::general_purpose::STANDARD //decode the encoded outgoing webhook event
                    .decode(last_event_body)
                    .unwrap();
                let decoded_str = String::from_utf8(decoded_bytes).unwrap();
                let webhook_response: HsWebhookResponse =
                    serde_json::from_str(&decoded_str).unwrap();
                if payment_id == webhook_response.content.object.payment_id
                    && webhook_status == webhook_response.content.object.status
                {
                    return Ok(());
                }
            }
            self.complete_actions(
                &web_driver,
                vec![Event::Trigger(Trigger::Sleep(webhook_retry_time))],
            )
            .await?;
        }
        Err(WebDriverError::CustomError("Webhook Not Found".to_string()))
    }
        /// Asynchronously makes a PayPal payment by executing a series of actions using the provided web driver and URL. 
    /// 
    /// # Arguments
    /// 
    /// * `web_driver` - The web driver to use for interacting with the web page.
    /// * `url` - The URL of the PayPal payment page.
    /// * `actions` - A vector of events representing the actions to be performed on the PayPal payment page.
    /// 
    /// # Returns
    /// 
    /// Returns a `Result` indicating success or an error of type `WebDriverError`.
    /// 
    async fn make_paypal_payment(
        &self,
        web_driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        let pypl_url = url.to_string();
        // To support failure retries
        let result = self
            .execute_paypal_steps(web_driver.clone(), &pypl_url, actions.clone())
            .await;
        if result.is_err() {
            self.execute_paypal_steps(web_driver.clone(), &pypl_url, actions.clone())
                .await
        } else {
            result
        }
    }
        /// Executes a series of PayPal automation steps using the provided WebDriver, URL, and list of events.
    ///
    /// # Arguments
    ///
    /// * `web_driver` - The WebDriver to interact with the web page.
    /// * `url` - The URL to navigate to before performing the PayPal automation steps.
    /// * `actions` - A list of events representing the automation steps to be executed after the PayPal-specific actions.
    ///
    /// # Returns
    ///
    /// * `Result<(), WebDriverError>` - A result indicating success or failure, with an optional error message.
    ///
    async fn execute_paypal_steps(
        &self,
        web_driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(
            &web_driver,
            vec![
                Event::Trigger(Trigger::Goto(url)),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
            ],
        )
        .await?;
        let (email, pass) = (
            &self
                .get_configs()
                .automation_configs
                .unwrap()
                .pypl_email
                .unwrap(),
            &self
                .get_configs()
                .automation_configs
                .unwrap()
                .pypl_pass
                .unwrap(),
        );
        let mut pypl_actions = vec![
            Event::Trigger(Trigger::Sleep(8)),
            Event::RunIf(
                Assert::IsPresentNow("Enter your email address to get started"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("email"), email)),
                    Event::Trigger(Trigger::Click(By::Id("btnNext"))),
                ],
            ),
            Event::RunIf(
                Assert::IsPresentNow("Password"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("password"), pass)),
                    Event::Trigger(Trigger::Click(By::Id("btnLogin"))),
                ],
            ),
        ];
        pypl_actions.extend(actions);
        self.complete_actions(&web_driver, pypl_actions).await
    }
        /// Asynchronously makes a Clearpay payment by completing a series of actions on the provided WebDriver instance.
    ///
    /// # Arguments
    ///
    /// * `driver` - The WebDriver instance to use for making the Clearpay payment.
    /// * `url` - The URL to navigate to in order to initiate the Clearpay payment process.
    /// * `actions` - Additional actions to be performed as part of the Clearpay payment process.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error of type WebDriverError.
    ///
    async fn make_clearpay_payment(
        &self,
        driver: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(
            &driver,
            vec![
                Event::Trigger(Trigger::Goto(url)),
                Event::Trigger(Trigger::Click(By::Id("card-submit-btn"))),
                Event::Trigger(Trigger::Sleep(5)),
                Event::RunIf(
                    Assert::IsPresentNow("Manage Cookies"),
                    vec![
                        Event::Trigger(Trigger::Click(By::Css("button.cookie-setting-link"))),
                        Event::Trigger(Trigger::Click(By::Id("accept-recommended-btn-handler"))),
                    ],
                ),
            ],
        )
        .await?;
        let (email, pass) = (
            &self
                .get_configs()
                .automation_configs
                .unwrap()
                .clearpay_email
                .unwrap(),
            &self
                .get_configs()
                .automation_configs
                .unwrap()
                .clearpay_pass
                .unwrap(),
        );
        let mut clearpay_actions = vec![
            Event::Trigger(Trigger::Sleep(3)),
            Event::EitherOr(
                Assert::IsPresent("Please enter your password"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Css("input[name='password']"), pass)),
                    Event::Trigger(Trigger::Click(By::Css("button[type='submit']"))),
                ],
                vec![
                    Event::Trigger(Trigger::SendKeys(
                        By::Css("input[name='identifier']"),
                        email,
                    )),
                    Event::Trigger(Trigger::Click(By::Css("button[type='submit']"))),
                    Event::Trigger(Trigger::Sleep(3)),
                    Event::Trigger(Trigger::SendKeys(By::Css("input[name='password']"), pass)),
                    Event::Trigger(Trigger::Click(By::Css("button[type='submit']"))),
                ],
            ),
            Event::Trigger(Trigger::Click(By::Css(
                "button[data-testid='summary-button']",
            ))),
        ];
        clearpay_actions.extend(actions);
        self.complete_actions(&driver, clearpay_actions).await
    }
}
/// Asynchronously checks if the specified text is currently present in the web page.
///
/// # Arguments
///
/// * `driver` - The WebDriver instance to use for finding the text.
/// * `key` - The text to search for in the web page.
///
/// # Returns
///
/// Returns a `WebDriverResult` with a boolean value indicating whether the text is present or not. If the text is present, it returns `true`, otherwise `false`. If the element containing the text is hidden, it returns an error.
async fn is_text_present_now(driver: &WebDriver, key: &str) -> WebDriverResult<bool> {
    let mut xpath = "//*[contains(text(),'".to_owned();
    xpath.push_str(key);
    xpath.push_str("')]");
    let result = driver.find(By::XPath(&xpath)).await?;
    let display: &str = &result.css_value("display").await?;
    if display.is_empty() || display == "none" {
        return Err(WebDriverError::CustomError("Element is hidden".to_string()));
    }
    result.is_present().await
}
///
/// Asynchronously checks if the specified text is present in the web page.
/// 
/// # Arguments
/// 
/// * `driver` - The WebDriver used to interact with the web page.
/// * `key` - The text to search for in the web page.
/// 
/// # Returns
/// 
/// A `WebDriverResult` containing a boolean value indicating whether the text is present in the web page or not.
/// 
async fn is_text_present(driver: &WebDriver, key: &str) -> WebDriverResult<bool> {
    let mut xpath = "//*[contains(text(),'".to_owned();
    xpath.push_str(key);
    xpath.push_str("')]");
    let result = driver.query(By::XPath(&xpath)).first().await?;
    result.is_present().await
}
/// Asynchronously checks if an element is present on the web page using the specified WebDriver instance and method of locating the element.
///
/// # Arguments
///
/// * `driver` - The WebDriver instance to use for querying the web page.
/// * `by` - The method of locating the element (e.g. by id, class, name, etc.).
///
/// # Returns
///
/// A `WebDriverResult` containing a boolean value indicating whether the element is present on the web page or not.
///
async fn is_element_present(driver: &WebDriver, by: By) -> WebDriverResult<bool> {
    let element = driver.query(by).first().await?;
    element.is_present().await
}

#[macro_export]
macro_rules! tester_inner {
    ($execute:ident, $webdriver:expr) => {{
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        let driver = $webdriver;

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let res = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let driver = runtime
                .block_on(driver)
                .expect("failed to construct test WebDriver");
            *sid.lock().unwrap() = runtime.block_on(driver.session_id()).ok();
            // make sure we close, even if an assertion fails
            let client = driver.clone();
            let x = runtime.block_on(async move {
                let run = tokio::spawn($execute(driver)).await;
                let _ = client.quit().await;
                run
            });
            drop(runtime);
            x.expect("test panicked")
        })
        .join();
        let success = handle_test_error(res);
        assert!(success);
    }};
}

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name.get(..name.len() - 3).unwrap()
    }};
}

#[macro_export]
macro_rules! tester {
    ($f:ident) => {{
        use $crate::{function, tester_inner};
        let test_name = format!("{:?}", function!());
        if (should_ignore_test(&test_name)) {
            return;
        }
        let browser = get_browser();
        let url = make_url(&browser);
        let caps = make_capabilities(&browser);
        tester_inner!($f, WebDriver::new(url, caps));
    }};
}

/// This method retrieves saved test cases from a file specified by the environment variable "CONNECTOR_TESTS_FILE_PATH". If the environment variable is not set, it returns an empty JSON value. It then reads the contents of the file, parses the JSON data, and returns the resulting serde_json::Value.
fn get_saved_testcases() -> serde_json::Value {
    let env_value = env::var("CONNECTOR_TESTS_FILE_PATH").ok();
    if env_value.is_none() {
        return serde_json::json!("");
    }
    let path = env_value.unwrap();
    let mut file = &std::fs::File::open(path).expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    // Parse the JSON data
    serde_json::from_str(&contents).expect("Failed to parse JSON")
}
/// Retrieves the connector authentication configurations from the file path specified in the environment variable "CONNECTOR_AUTH_FILE_PATH".
/// 
/// # Panics
/// 
/// This method will panic if the "CONNECTOR_AUTH_FILE_PATH" environment variable is not set, or if the file specified by the path is not found, or if the connector authentication config file cannot be read or parsed.
fn get_configs() -> connector_auth::ConnectorAuthentication {
    let path =
        env::var("CONNECTOR_AUTH_FILE_PATH").expect("connector authentication file path not set");
    toml::from_str(
        &std::fs::read_to_string(path).expect("connector authentication config file not found"),
    )
    .expect("Failed to read connector authentication config file")
}

/// Checks if a test should be ignored based on the test name. 
pub fn should_ignore_test(name: &str) -> bool {
    let conf = get_saved_testcases()
        .get("tests_to_ignore")
        .unwrap_or(&json!([]))
        .clone();
    let tests_to_ignore: HashSet<String> =
        serde_json::from_value(conf).unwrap_or_else(|_| HashSet::new());
    let modules: Vec<_> = name.split("::").collect();
    let file_match = format!(
        "{}::*",
        <&str>::clone(modules.get(1).expect("Error obtaining module path segment"))
    );
    let module_name = modules
        .get(1..3)
        .expect("Error obtaining module path segment")
        .join("::");
    // Ignore if it matches patterns like nuvei_ui::*, nuvei_ui::should_make_nuvei_eps_payment_test
    tests_to_ignore.contains(&file_match) || tests_to_ignore.contains(&module_name)
}

/// Returns the name of the browser as a String.
pub fn get_browser() -> String {
    "firefox".to_string()
}

// based on the browser settings build profile info
pub fn make_capabilities(browser: &str) -> Capabilities {
    match browser {
        "firefox" => {
            let mut caps = DesiredCapabilities::firefox();
            let ignore_profile = env::var("IGNORE_BROWSER_PROFILE").ok();
            if ignore_profile.is_none() || ignore_profile.unwrap() == "false" {
                let profile_path = &format!("-profile={}", get_firefox_profile_path().unwrap());
                caps.add_firefox_arg(profile_path).unwrap();
            } else {
                let profile_path = &format!("-profile={}", get_firefox_profile_path().unwrap());
                caps.add_firefox_arg(profile_path).unwrap();
                caps.add_firefox_arg("--headless").ok();
            }
            caps.into()
        }
        "chrome" => {
            let mut caps = DesiredCapabilities::chrome();
            let profile_path = &format!("user-data-dir={}", get_chrome_profile_path().unwrap());
            caps.add_chrome_arg(profile_path).unwrap();
            caps.into()
        }
        &_ => DesiredCapabilities::safari().into(),
    }
}

/// This method retrieves the path to the user's Chrome profile directory. It first gets the current executable's path, and then constructs the base path by taking the parent directory and truncating it to the first three components. On macOS, it appends the path to the default Chrome profile directory. It then returns the base path as a `Result` with the profile path as `Ok` or a `WebDriverError` as `Err`.
fn get_chrome_profile_path() -> Result<String, WebDriverError> {
    let exe = env::current_exe()?;
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut base_path = dir
        .to_str()
        .map(|str| {
            let mut fp = str.split(MAIN_SEPARATOR).collect::<Vec<_>>();
            fp.truncate(3);
            fp.join(&MAIN_SEPARATOR.to_string())
        })
        .unwrap();
    if env::consts::OS == "macos" {
        base_path.push_str(r"/Library/Application\ Support/Google/Chrome/Default");
        //Issue: 1573
    } // We're only using Firefox on Ubuntu runner
    Ok(base_path)
}

/// Retrieves the path to the Firefox profile directory based on the current operating system.
/// Returns a Result containing the path as a String if successful, or a WebDriverError if an error occurs.
fn get_firefox_profile_path() -> Result<String, WebDriverError> {
    let exe = env::current_exe()?;
    let dir = exe.parent().expect("Executable must be in some directory");
    let mut base_path = dir
        .to_str()
        .map(|str| {
            let mut fp = str.split(MAIN_SEPARATOR).collect::<Vec<_>>();
            fp.truncate(3);
            fp.join(&MAIN_SEPARATOR.to_string())
        })
        .unwrap();
    if env::consts::OS == "macos" {
        base_path.push_str(r#"/Library/Application Support/Firefox/Profiles/hs-test"#);
    //Issue: 1573
    } else if env::consts::OS == "linux" {
        if let Some(home_dir) = env::var_os("HOME") {
            if let Some(home_path) = home_dir.to_str() {
                let profile_path = format!("{}/.mozilla/firefox/hs-test", home_path);
                return Ok(profile_path);
            }
        }
    }
    Ok(base_path)
}

/// Given a browser name, returns the corresponding WebDriver URL for that browser.
pub fn make_url(browser: &str) -> &'static str {
    match browser {
        "firefox" => "http://localhost:4444",
        "chrome" => "http://localhost:9515",
        &_ => "",
    }
}

/// This method takes a Result type as input and checks its content. If the inner Result is Ok with a nested Ok value, it returns true. If the inner Result is Ok with a nested Err value, it prints an error message and returns false. If the outer Result is Err, it checks if the error can be downcasted to a specific type and prints a corresponding error message. Finally, it returns false.
pub fn handle_test_error(
    res: Result<Result<(), WebDriverError>, Box<dyn std::any::Any + Send>>,
) -> bool {
    match res {
        Ok(Ok(_)) => true,
        Ok(Err(web_driver_error)) => {
            eprintln!("test future failed to resolve: {:?}", web_driver_error);
            false
        }
        Err(e) => {
            if let Some(web_driver_error) = e.downcast_ref::<WebDriverError>() {
                eprintln!("test future panicked: {:?}", web_driver_error);
            } else {
                eprintln!("test future panicked; an assertion probably failed");
            }
            false
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    data: Vec<WebhookResponseData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponseData {
    step: WebhookRequestData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookRequestData {
    request: WebhookRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookRequest {
    body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HsWebhookResponse {
    content: HsWebhookContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HsWebhookContent {
    object: HsWebhookObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HsWebhookObject {
    payment_id: String,
    status: String,
    connector: String,
}
