#![allow(dead_code)]

use ajou_parser::MY_USER_AGENT;
use anyhow::{anyhow, Result};
use chromiumoxide::{
    fetcher::BrowserFetcherRevisionInfo, handler::viewport::Viewport, Browser, BrowserConfig,
    BrowserFetcher, BrowserFetcherOptions, Page,
};
use dotenv::dotenv;
use mongodb::{
    bson::{doc, to_bson},
    options::{ClientOptions, UpdateOptions},
    Client,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;
use tracing::debug;

fn get_collection_name(category: &str) -> String {
    format!("course_2023-1_{}", category)
}

async fn insert_courses_to_mongodb(category: &str, courses: Vec<Course>) -> Result<()> {
    let connection_uri = std::env::var("MONGODB").expect("MONGODB must be set.");
    let client_options = ClientOptions::parse(connection_uri).await?;
    let client = Client::with_options(client_options)?;

    let database_name = "ajou";
    let collection_name = get_collection_name(category);
    println!("Inserting courses for {collection_name}...");

    let db = client.database(database_name);
    let collection = db.collection::<Course>(&collection_name);

    for course in courses {
        // Assuming 'subject_code' is unique for each course
        let filter = doc! { "subject_code": &course.subject_id };

        let document = to_bson(&course)?
            .as_document()
            .ok_or_else(|| anyhow!("Error converting course to BSON document"))?
            .clone();

        let update = doc! { "$set": document };
        let options = UpdateOptions::builder().upsert(true).build();

        collection
            .update_one(filter, update, options)
            .await
            .map_err(|e| anyhow!("Error upserting course in MongoDB: {:?}", e))?;

        // println!("Updated documents: {:?}", result.upserted_id);
    }

    println!("Finished updating courses...");
    Ok(())
}

// Course
#[derive(Debug, Deserialize, Default)]
pub struct VariableList {
    #[serde(rename = "ErrorMsg")]
    error_msg: String,
    #[serde(rename = "ErrorCode")]
    error_code: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Course {
    #[serde(rename(deserialize = "tm", serialize = "duration"))]
    duration: f32,

    #[serde(rename(deserialize = "submattFgEngNm", serialize = "course_type"))]
    course_type: Option<String>,

    #[serde(rename(deserialize = "ltTmEngNm", serialize = "class_time"))]
    class_time: String,

    #[serde(rename(deserialize = "clssNo", serialize = "class_number"))]
    class_number: String,

    #[serde(
        rename(deserialize = "rcomShyrCdNm", serialize = "recommended_year"),
        default
    )]
    recommended_year: Option<String>,

    #[serde(rename(deserialize = "sustLsnFgNm", serialize = "course_category"))]
    course_category: String,

    #[serde(rename(
        deserialize = "maLecturerEmplNo",
        serialize = "main_lecturer_employee_number"
    ))]
    main_lecturer_employee_number: String,

    #[serde(rename(
        deserialize = "abeekInspPracPntCnt",
        serialize = "abeek_practical_points"
    ))]
    abeek_practical_points: f32,

    #[serde(rename(deserialize = "fileNm", serialize = "file_name"), default)]
    file_name: Option<String>,

    #[serde(rename(deserialize = "maLecturerEmplNm", serialize = "main_lecturer_name"))]
    main_lecturer_name: String,

    #[serde(rename(deserialize = "sustLsnFgEngNm", serialize = "course_category_english"))]
    course_category_english: String,

    #[serde(
        rename(deserialize = "mjCdEngNm", serialize = "major_code_english"),
        default
    )]
    major_code_english: Option<String>,

    #[serde(rename(deserialize = "sustCd", serialize = "department_code"))]
    department_code: String,

    #[serde(rename(deserialize = "planInputYn", serialize = "plan_input_status"))]
    plan_input_status: String,

    #[serde(rename(deserialize = "filePath", serialize = "file_path"), default)]
    file_path: Option<String>,

    #[serde(rename(
        deserialize = "abeekTheoPntCnt",
        serialize = "abeek_theoretical_points"
    ))]
    abeek_theoretical_points: f32,

    #[serde(rename(deserialize = "ltRoomEngNm", serialize = "classroom_english"))]
    classroom_english: String,

    #[serde(rename(deserialize = "emplNo", serialize = "employee_number"))]
    employee_number: String,

    #[serde(
        rename(deserialize = "sustCdEngNm", serialize = "department_english"),
        default
    )]
    department_english: Option<String>,

    #[serde(rename(deserialize = "submattFgNm", serialize = "course_type_korean"))]
    course_type_korean: String,

    #[serde(rename(deserialize = "sbjtCd", serialize = "subject_code"))]
    subject_code: String,

    #[serde(rename(deserialize = "mainOpenLtNo", serialize = "main_open_course_number"))]
    main_open_course_number: String,

    #[serde(rename(deserialize = "mjCd", serialize = "major_code"))]
    major_code: String,

    #[serde(rename(deserialize = "mjCdNm", serialize = "major_name"), default)]
    major_name: Option<String>,

    #[serde(rename(deserialize = "ltRoomNm", serialize = "classroom"))]
    classroom: String,

    #[serde(rename(deserialize = "abeePnt", serialize = "abee_point"))]
    abee_point: Option<f32>,

    #[serde(rename(deserialize = "shtmCd", serialize = "semester_code"))]
    semester_code: String,

    #[serde(rename(
        deserialize = "maLecturerEmplEngNm",
        serialize = "main_lecturer_english_name"
    ))]
    main_lecturer_english_name: Option<String>,

    #[serde(rename(deserialize = "sustLsnFg", serialize = "course_category_code"))]
    course_category_code: String,

    #[serde(rename(deserialize = "openLtNo", serialize = "open_course_number"))]
    open_course_number: String,

    #[serde(rename(deserialize = "orgLangLtYn", serialize = "original_language_course"))]
    original_language_course: Option<String>,

    #[serde(rename(deserialize = "cqiYn", serialize = "cqi_status"))]
    cqi_status: String,

    #[serde(rename(deserialize = "lsnApprDetailPop", serialize = "course_evaluation"))]
    course_evaluation: String,

    #[serde(rename(deserialize = "shtmNm", serialize = "semester_name"))]
    semester_name: String,

    #[serde(rename(deserialize = "yy", serialize = "year"))]
    year: String,

    #[serde(rename(deserialize = "sustCdNm", serialize = "department_name"))]
    department_name: Option<String>,

    #[serde(
        rename(deserialize = "engGrdFgNm", serialize = "english_grade_type"),
        default
    )]
    english_grade_type: Option<String>,

    #[serde(rename(deserialize = "abeekDgnPntCnt", serialize = "abeek_design_points"))]
    abeek_design_points: f32,

    #[serde(rename(deserialize = "abeekYn", serialize = "abeek_status"))]
    abeek_status: String,

    #[serde(rename(deserialize = "fileFg", serialize = "file_status"))]
    file_status: String,

    #[serde(rename(deserialize = "sbjtKorNm", serialize = "subject_korean_name"))]
    subject_korean_name: String,

    #[serde(
        rename(
            deserialize = "lsnPdocMngtClssYn",
            serialize = "lesson_document_management_class"
        ),
        default
    )]
    lesson_document_management_class: Option<String>,

    #[serde(rename(deserialize = "ltTmNm", serialize = "class_time_korean"))]
    class_time_korean: String,

    #[serde(
        rename(deserialize = "rcomShyrCd", serialize = "recommended_year_code"),
        default
    )]
    recommended_year_code: Option<String>,

    #[serde(rename(deserialize = "tlsnNo", serialize = "lesson_number"))]
    lesson_number: String,

    #[serde(rename(deserialize = "apprUnAdptYn", serialize = "approved_unadapted"))]
    approved_unadapted: String,

    #[serde(rename(deserialize = "pnt", serialize = "credit_points"))]
    credit_points: f32,

    #[serde(rename(deserialize = "sbjtId", serialize = "subject_id"))]
    subject_id: String,

    #[serde(rename(deserialize = "sbjtEngNm", serialize = "subject_english_name"))]
    subject_english_name: String,

    #[serde(rename(deserialize = "coopOpenLtYn", serialize = "cooperative_open_course"))]
    cooperative_open_course: String,

    #[serde(
        rename(deserialize = "coopLt", serialize = "cooperative_course"),
        default
    )]
    cooperative_course: Option<String>,

    #[serde(rename(deserialize = "rowStatus", serialize = "row_status"))]
    row_status: i32,

    #[serde(
        rename(deserialize = "ltFgNm", serialize = "lecture_type_name"),
        default
    )]
    lecture_type_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DatasetList {
    #[serde(rename = "DS_COUR120")]
    pub ds_cour120: Vec<Course>,
}

#[derive(Debug, Deserialize, Default)]

pub struct CourseResp {
    #[serde(rename = "VariableList")]
    pub var_list: VariableList,
    #[serde(rename = "DatasetList")]
    pub data_list: DatasetList,
}

async fn course_parse(str_submatt_fg: &str, jsession: &str) -> Result<CourseResp, reqwest::Error> {
    println!("Course parse: {}", str_submatt_fg);
    let payload = serde_json::json!({
        "url": "uni/uni/cour/lssn/findCourLecturePlanDocumentReg.action",
        "param": {
            "strYy": "2023",
            "strShtmCd": "U0002001",
            "strSubmattFg": str_submatt_fg,
            "strSustcd": "",
            "strMjCd": "",
            "strSubmattFldFg": "",
            "strCoopOpenYn": "공동개설"
        }
    });

    // Create a HeaderMap and insert the necessary headers
    let jsession_id = format!("JSESSIONID={jsession};");

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&jsession_id)
            .expect("Failed to create header value from jsession_id"),
    );

    let client: reqwest::Client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(ajou_notice::MY_USER_AGENT)
        .build()
        .unwrap();

    let res = client
        .post(std::env::var("COURSE").unwrap())
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let body = res.text().await?;

    let courses: Result<CourseResp, serde_json::Error> = serde_json::from_str(&body);
    match courses {
        Ok(courses) => Ok(courses),
        Err(_) => Ok(CourseResp::default()),
    }
}

/// Starts the browser and returns a handle to it.
///
/// # Arguments
///
/// * `browser_path` - The path to the browser executable (will be downloaded if not found).
/// * `user_data_dir` - The path to the user data directory (will be created if not found).
/// * `headless` - Whether to run the browser in headless mode.
///
/// # Errors
///
/// * If the browser executable cannot be found or downloaded.
/// * If the user data directory cannot be created.
/// * If the browser cannot be launched.
/// * If the browser handler cannot be spawned.
pub async fn init_browser(
    browser_path: &Path,
    user_data_dir: &Path,
    headless: bool,
) -> Result<Browser> {
    fs::create_dir_all(browser_path)?;
    fs::create_dir_all(user_data_dir)?;

    let browser_info = ensure_browser(browser_path).await?;

    let mut viewport = Viewport::default();
    viewport.width = 1440;
    viewport.height = 900;

    let mut config = BrowserConfig::builder()
        .user_data_dir(user_data_dir)
        .chrome_executable(browser_info.executable_path)
        .with_head()
        .no_sandbox()
        .viewport(viewport)
        .window_size(1440, 900)
        .args(vec![format!("--user-agent={}", MY_USER_AGENT)]);

    if headless {
        config = config.arg("--headless");
    }

    let (browser, mut handler) = Browser::launch(config.build().map_err(|e| anyhow!(e))?).await?;

    tokio::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                debug!("Browser handler error: {:?}", h);
                break;
            }
        }
    });

    Ok(browser)
}

async fn ensure_browser(path: &Path) -> Result<BrowserFetcherRevisionInfo> {
    let fetcher = BrowserFetcher::new(BrowserFetcherOptions::builder().with_path(path).build()?);

    Ok(fetcher.fetch().await?)
}

/// Waits for the page to navigate or for 5 seconds to pass.
///
/// # Arguments
///
/// * `page` - The page to wait for.
pub async fn wait_for_page(page: &Page) {
    tokio::select! {
        _ = page.wait_for_navigation() => {},
        _ = sleep(Duration::from_secs(5)) => {},
    }
}

async fn get_jsession_id(page: &Page) -> Result<Option<String>> {
    let cookies = page.get_cookies().await?;
    // println!("{:#?}", cookies);
    let jsessionid_value = cookies
        .iter()
        .find(|cookie| cookie.name == "JSESSIONID")
        .map(|cookie| cookie.value.clone());

    Ok(jsessionid_value)
}

async fn wait_for_url(page: &Page, target_url: &str, timeout: Duration) -> Result<bool> {
    let start_time = tokio::time::Instant::now();
    loop {
        if page.url().await?.unwrap().eq(target_url) {
            return Ok(true);
        }
        if start_time.elapsed() > timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn login_if_needed(page: &Page) -> Result<()> {
    if let Ok(user_id_element) = page.find_element("input#userId").await {
        let id = std::env::var("ID").expect("ID must be set.");
        let pw = std::env::var("PASSWORD").expect("PASSWORD must be set.");

        user_id_element.click().await?.type_str(&id).await?;

        page.find_element("input#password")
            .await?
            .click()
            .await?
            .type_str(&pw)
            .await?;

        page.find_element("a#loginSubmit").await?.click().await?;

        // wait_for_url(
        //     &page,
        //     "https://mhaksa.ajou.ac.kr:30443/index.html",
        //     Duration::from_secs(5),
        // )
        // .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let mut browser = init_browser(Path::new("./browser"), Path::new("./user_data"), true).await?;

    let page = browser
        .new_page("https://mhaksa.ajou.ac.kr:30443/index.html")
        .await?;

    if wait_for_url(
        &page,
        "https://sso.ajou.ac.kr/jsp/sso/ip/login_form.jsp",
        Duration::from_secs(5),
    )
    .await?
    {
        login_if_needed(&page).await?;
    }

    wait_for_url(
        &page,
        "https://mhaksa.ajou.ac.kr:30443/index.html",
        Duration::from_secs(5),
    )
    .await?;

    let mut jsession = match get_jsession_id(&page).await {
        Ok(jsession_value) => jsession_value.unwrap(),
        Err(_) => {
            browser.close().await?;
            panic!("JSESSIONID not found")
        }
    };

    while !jsession.contains("chusa_servlet_HAKSA01") {
        wait_for_url(
            &page,
            "https://mhaksa.ajou.ac.kr:30443/index.html",
            Duration::from_secs(1),
        )
        .await?;
        jsession = get_jsession_id(&page).await?.unwrap();
    }

    browser.close().await?;

    println!("Browser closed");

    let course = course_parse("U0209001", &jsession).await.unwrap();
    insert_courses_to_mongodb("전공과목", course.data_list.ds_cour120).await?; // 전공과목 전체

    let course = course_parse("U0209002", &jsession).await.unwrap();
    insert_courses_to_mongodb("교양과목", course.data_list.ds_cour120).await?; // 교양과목 전체

    let course = course_parse("U0209003", &jsession).await.unwrap();
    insert_courses_to_mongodb("기초과목", course.data_list.ds_cour120).await?; // 기초과목 공통

    let course = course_parse("U0209004", &jsession).await.unwrap();
    insert_courses_to_mongodb("공학기초", course.data_list.ds_cour120).await?; // 공학기초 전체

    let course = course_parse("U0209005", &jsession).await.unwrap();
    insert_courses_to_mongodb("영역별교양", course.data_list.ds_cour120).await?; // 영역별교양 전체

    let course = course_parse("U0209006", &jsession).await.unwrap();
    insert_courses_to_mongodb("학점교류", course.data_list.ds_cour120).await?; // 학점교류 전체

    let course = course_parse("U0209029", &jsession).await.unwrap();
    insert_courses_to_mongodb("일선과목", course.data_list.ds_cour120).await?; // 일선과목 전체

    Ok(())
}

#[tokio::test]
async fn course_test() {
    let course = course_parse("U0209005", "").await.unwrap();

    println!("{:#?}", course.data_list.ds_cour120[0]);
}
