use std::{collections::HashMap, env, path::MAIN_SEPARATOR, time::Duration};

use actix_web::cookie::SameSite;
use async_trait::async_trait;
use futures::Future;
use thirtyfour::{components::SelectElement, prelude::*, WebDriver};

pub enum Event<'a> {
    RunIf(Assert<'a>, Vec<Event<'a>>),
    EitherOr(Assert<'a>, Vec<Event<'a>>, Vec<Event<'a>>),
    Assert(Assert<'a>),
    Trigger(Trigger<'a>),
}

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

pub enum Position {
    Prev,
    Next,
}
pub enum Selector {
    Title,
    QueryParamStr,
}

pub enum Assert<'a> {
    Eq(Selector, &'a str),
    Contains(Selector, &'a str),
    IsPresent(&'a str),
}

#[async_trait]
pub trait SeleniumTest {
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
                    Assert::Eq(_selector, text) => assert_eq!(driver.title().await?, text),
                    Assert::IsPresent(text) => {
                        assert!(is_text_present(driver, text).await?)
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
                    Assert::Eq(_selector, text) => {
                        if text == driver.title().await? {
                            self.complete_actions(driver, events).await?;
                        }
                    }
                    Assert::IsPresent(text) => {
                        if is_text_present(driver, text).await.is_ok() {
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
                },
                Event::Trigger(trigger) => match trigger {
                    Trigger::Goto(url) => {
                        driver.goto(url).await?;
                        let hs_base_url =
                            env::var("HS_BASE_URL").unwrap_or("http://localhost:8080".to_string());
                        let hs_api_key =
                            env::var("HS_API_KEY").expect("Hyperswitch user API key not present");
                        driver
                            .add_cookie(new_cookie("hs_base_url", hs_base_url).clone())
                            .await?;
                        driver
                            .add_cookie(new_cookie("hs_api_key", hs_api_key).clone())
                            .await?;
                    }
                    Trigger::Click(by) => {
                        let ele = driver.query(by).first().await?;
                        ele.wait_until().displayed().await?;
                        ele.wait_until().clickable().await?;
                        ele.click().await?;
                    }
                    Trigger::ClickNth(by, n) => {
                        let ele = driver.query(by).all().await?.into_iter().nth(n).unwrap();
                        ele.wait_until().displayed().await?;
                        ele.wait_until().clickable().await?;
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
                            if let Some(window) = windows.iter().rev().next() {
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

    async fn process_payment<F, Fut>(&self, _f: F) -> Result<(), WebDriverError>
    where
        F: FnOnce(WebDriver) -> Fut + Send,
        Fut: Future<Output = Result<(), WebDriverError>> + Send,
    {
        let _browser = env::var("HS_TEST_BROWSER").unwrap_or("chrome".to_string());
        Ok(())
    }
    async fn make_redirection_payment(
        &self,
        c: WebDriver,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(&c, actions).await
    }
    async fn make_gpay_payment(
        &self,
        c: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        let (email, pass) = (
            &get_env("GMAIL_EMAIL").clone(),
            &get_env("GMAIL_PASS").clone(),
        );
        let default_actions = vec![
            Event::Trigger(Trigger::Goto(url)),
            Event::Trigger(Trigger::Click(By::Css("#gpay-btn button"))),
            Event::Trigger(Trigger::SwitchTab(Position::Next)),
            Event::RunIf(
                Assert::IsPresent("Sign in"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("identifierId"), email)),
                    Event::Trigger(Trigger::ClickNth(By::Tag("button"), 2)),
                    Event::EitherOr(
                        Assert::IsPresent("Welcome"),
                        vec![
                            Event::Trigger(Trigger::SendKeys(By::Name("Passwd"), pass)),
                            Event::Trigger(Trigger::Sleep(2)),
                            Event::Trigger(Trigger::Click(By::Id("passwordNext"))),
                        ],
                        vec![
                            Event::Trigger(Trigger::SendKeys(By::Id("identifierId"), email)),
                            Event::Trigger(Trigger::ClickNth(By::Tag("button"), 2)),
                            Event::Trigger(Trigger::SendKeys(By::Name("Passwd"), pass)),
                            Event::Trigger(Trigger::Sleep(2)),
                            Event::Trigger(Trigger::Click(By::Id("passwordNext"))),
                        ],
                    ),
                ],
            ),
            Event::Trigger(Trigger::Query(By::ClassName(
                "bootstrapperIframeContainerElement",
            ))),
            Event::Trigger(Trigger::SwitchFrame(By::Id("sM432dIframe"))),
            Event::Assert(Assert::IsPresent("Gpay Tester")),
            Event::Trigger(Trigger::Click(By::ClassName("jfk-button-action"))),
            Event::Trigger(Trigger::SwitchTab(Position::Prev)),
        ];
        self.complete_actions(&c, default_actions).await?;
        self.complete_actions(&c, actions).await
    }
    async fn make_paypal_payment(
        &self,
        c: WebDriver,
        url: &str,
        actions: Vec<Event<'_>>,
    ) -> Result<(), WebDriverError> {
        self.complete_actions(
            &c,
            vec![
                Event::Trigger(Trigger::Goto(url)),
                Event::Trigger(Trigger::Click(By::Id("pypl-redirect-btn"))),
            ],
        )
        .await?;
        let (email, pass) = (
            &get_env("PYPL_EMAIL").clone(),
            &get_env("PYPL_PASS").clone(),
        );
        let mut pypl_actions = vec![
            Event::EitherOr(
                Assert::IsPresent("Password"),
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("password"), pass)),
                    Event::Trigger(Trigger::Click(By::Id("btnLogin"))),
                ],
                vec![
                    Event::Trigger(Trigger::SendKeys(By::Id("email"), email)),
                    Event::Trigger(Trigger::Click(By::Id("btnNext"))),
                    Event::Trigger(Trigger::SendKeys(By::Id("password"), pass)),
                    Event::Trigger(Trigger::Click(By::Id("btnLogin"))),
                ],
            ),
            Event::Trigger(Trigger::Click(By::Id("payment-submit-btn"))),
        ];
        pypl_actions.extend(actions);
        self.complete_actions(&c, pypl_actions).await
    }
}
async fn is_text_present(driver: &WebDriver, key: &str) -> WebDriverResult<bool> {
    let mut xpath = "//*[contains(text(),'".to_owned();
    xpath.push_str(key);
    xpath.push_str("')]");
    let result = driver.query(By::XPath(&xpath)).first().await?;
    result.is_present().await
}
fn new_cookie(name: &str, value: String) -> Cookie<'_> {
    let mut base_url_cookie = Cookie::new(name, value);
    base_url_cookie.set_same_site(Some(SameSite::Lax));
    base_url_cookie.set_domain("hs-payment-tests.w3spaces.com");
    base_url_cookie.set_path("/");
    base_url_cookie
}

#[macro_export]
macro_rules! tester_inner {
    ($f:ident, $connector:expr) => {{
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        let c = $connector;

        // we'll need the session_id from the thread
        // NOTE: even if it panics, so can't just return it
        let session_id = Arc::new(Mutex::new(None));

        // run test in its own thread to catch panics
        let sid = session_id.clone();
        let res = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let c = rt.block_on(c).expect("failed to construct test WebDriver");
            *sid.lock().unwrap() = rt.block_on(c.session_id()).ok();
            // make sure we close, even if an assertion fails
            let client = c.clone();
            let x = rt.block_on(async move {
                let r = tokio::spawn($f(c)).await;
                let _ = client.quit().await;
                r
            });
            drop(rt);
            x.expect("test panicked")
        })
        .join();
        let success = handle_test_error(res);
        assert!(success);
    }};
}

#[macro_export]
macro_rules! tester {
    ($f:ident, $endpoint:expr) => {{
        use $crate::tester_inner;

        let url = make_url($endpoint);
        let caps = make_capabilities($endpoint);
        tester_inner!($f, WebDriver::new(url, caps));
    }};
}

pub fn make_capabilities(s: &str) -> Capabilities {
    match s {
        "firefox" => {
            let mut caps = DesiredCapabilities::firefox();
            let profile_path = &format!("-profile={}", get_firefox_profile_path().unwrap());
            caps.add_firefox_arg(profile_path).unwrap();
            // let mut prefs = FirefoxPreferences::new();
            // prefs.set("-browser.link.open_newwindow", 3).unwrap();
            // caps.set_preferences(prefs).unwrap();
            caps.into()
        }
        "chrome" => {
            let mut caps = DesiredCapabilities::chrome();
            let profile_path = &format!("user-data-dir={}", get_chrome_profile_path().unwrap());
            caps.add_chrome_arg(profile_path).unwrap();
            // caps.set_headless().unwrap();
            // caps.set_no_sandbox().unwrap();
            // caps.set_disable_gpu().unwrap();
            // caps.set_disable_dev_shm_usage().unwrap();
            caps.into()
        }
        &_ => DesiredCapabilities::safari().into(),
    }
}
fn get_chrome_profile_path() -> Result<String, WebDriverError> {
    env::var("CHROME_PROFILE_PATH").map_or_else(
        |_| -> Result<String, WebDriverError> {
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
            base_path.push_str(r#"/Library/Application\ Support/Google/Chrome/Default"#);
            Ok(base_path)
        },
        Ok,
    )
}
fn get_firefox_profile_path() -> Result<String, WebDriverError> {
    env::var("FIREFOX_PROFILE_PATH").map_or_else(
        |_| -> Result<String, WebDriverError> {
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
            base_path.push_str(r#"/Library/Application Support/Firefox/Profiles/hs-test"#);
            Ok(base_path)
        },
        Ok,
    )
}

pub fn make_url(s: &str) -> &'static str {
    match s {
        "firefox" => "http://localhost:4444",
        "chrome" => "http://localhost:9515",
        &_ => "",
    }
}

pub fn handle_test_error(
    res: Result<Result<(), WebDriverError>, Box<dyn std::any::Any + Send>>,
) -> bool {
    match res {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => {
            eprintln!("test future failed to resolve: {:?}", e);
            false
        }
        Err(e) => {
            if let Some(e) = e.downcast_ref::<WebDriverError>() {
                eprintln!("test future panicked: {:?}", e);
            } else {
                eprintln!("test future panicked; an assertion probably failed");
            }
            false
        }
    }
}

pub fn get_env(name: &str) -> String {
    env::var(name)
        .unwrap_or_else(|_| panic!("{name} not present"))
}
