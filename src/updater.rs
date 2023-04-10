use ajou_parser::{Notice, AJOU_LINK, MY_USER_AGENT};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Asia::Seoul;
use chrono_tz::Tz;
use dotenv::dotenv;
use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::{options::ClientOptions, options::FindOptions, Client};
use reqwest::header::USER_AGENT;
use scraper::{Html, Selector};
use std::borrow::Cow;
use tokio::time::{sleep, Duration};

const DEFAULT_NUM_ARTICLES: usize = 7;

fn get_query(query_option: &str) -> Cow<'_, str> {
    match query_option {
        "ajou" => "?mode=list&article.offset=0&articleLimit=".into(),
        _ => format!(
            "?mode=list&srSearchKey=&srSearchVal={}&article.offset=0&articleLimit=",
            query_option
        )
        .into(),
    }
}

pub async fn notice_parse(
    query_option: &str,
    _nums: Option<usize>,
) -> Result<Vec<Notice>, reqwest::Error> {
    let query = get_query(query_option);
    let nums_int = _nums.unwrap_or(DEFAULT_NUM_ARTICLES);

    let url = [AJOU_LINK, &query, &nums_int.to_string()].concat();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    // header 없이 보내면 404
    let res = client
        .get(url)
        .header(USER_AGENT, MY_USER_AGENT)
        .send()
        .await?;
    let body = res.text().await?;

    // HTML Parse
    let document = Html::parse_document(&body);
    let a_selector = Selector::parse("a").unwrap();

    let ids = Selector::parse("td.b-num-box").unwrap();
    let cates = Selector::parse("span.b-cate").unwrap();
    let titles = Selector::parse("div.b-title-box").unwrap();
    let dates = Selector::parse("span.b-date").unwrap();
    let writers = Selector::parse("span.b-writer").unwrap();

    let id_elements = document.select(&ids);
    let mut cate_elements = document.select(&cates);
    let mut title_elements = document.select(&titles);
    let mut date_elements = document.select(&dates);
    let mut writer_elements = document.select(&writers);

    let mut notices: Vec<Notice> = id_elements
        .filter_map(|id_element| {
            let date = date_elements.next()?.text().next()?.trim().to_string();
            let writer = writer_elements
                .next()?
                .text()
                .next()
                .unwrap_or("알 수 없음")
                .trim()
                .to_string();
            let category = cate_elements.next()?.text().next()?.trim().to_string();
            let inner_a = title_elements.next()?.select(&a_selector).next()?;
            let id = id_element.text().next()?.trim().parse::<i32>().ok()?;

            let mut title = inner_a.value().attr("title")?.to_string();
            let link = format!("{}{}", AJOU_LINK, inner_a.value().attr("href").unwrap());

            let dup = format!("[{}]", writer);
            if title.contains(&dup) {
                title = title.replace(&dup, "");
            }

            title = title
                .replace(" 자세히 보기", "")
                .replace("(재공지)", "")
                .trim()
                .to_string();

            Some(Notice {
                id,
                category,
                title,
                link,
                date,
                writer,
            })
        })
        .collect();

    notices.reverse();

    Ok(notices)
}

async fn update_database(
    notice_collection: &mongodb::Collection<Notice>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rust_notices: Vec<Notice> = Vec::new();

    let find_options = FindOptions::builder()
        .sort(doc! { "id": -1})
        .limit(1)
        .build();

    let mut db_notices = notice_collection.find(doc! {}, find_options).await.unwrap();

    while let Some(notice) = db_notices.try_next().await.unwrap() {
        rust_notices.push(notice);
    }
    let parsed_notice = notice_parse("ajou", Some(1)).await.unwrap();

    let last_db_notice_id = rust_notices.first().unwrap().id;
    let last_parsed_notice_id = parsed_notice.first().unwrap().id;

    let num_missing_notices: usize = (last_parsed_notice_id - last_db_notice_id) as usize;

    let parsed_notices = if num_missing_notices != 0 {
        notice_parse("ajou", Some(num_missing_notices))
            .await
            .unwrap()
    } else {
        vec![]
    };

    if !parsed_notices.is_empty() {
        notice_collection
            .insert_many(parsed_notices, None)
            .await
            .unwrap();
    }
    Ok(())
}

async fn wait_until_next_morning(seoul_now: DateTime<Tz>) {
    // let mut next_morning = Seoul
    //     .ymd(seoul_now.year(), seoul_now.month(), seoul_now.day())
    //     .and_hms(9, 30, 0);
    let mut next_morning = Seoul
        .with_ymd_and_hms(
            seoul_now.year(),
            seoul_now.month(),
            seoul_now.day(),
            9,
            30,
            0,
        )
        .unwrap();

    if seoul_now.hour() >= 19 {
        next_morning = next_morning + chrono::Duration::days(1);
    }

    let difference = (next_morning - seoul_now).num_seconds();

    println!(
        "Night time...resting until next KST 9am: {} seconds",
        difference
    );
    sleep(Duration::from_secs(difference as u64)).await;
}

async fn wait_until_weekday(seoul_now: DateTime<Tz>) {
    // let monday = Seoul
    //     .ymd(seoul_now.year(), seoul_now.month(), seoul_now.day())
    //     .and_hms(9, 0, 0)
    //     + chrono::Duration::days(7 - seoul_now.weekday().num_days_from_monday() as i64);

    let monday = Seoul
        .with_ymd_and_hms(
            seoul_now.year(),
            seoul_now.month(),
            seoul_now.day(),
            9,
            0,
            0,
        )
        .unwrap()
        + chrono::Duration::days(7 - seoul_now.weekday().num_days_from_monday() as i64);

    let difference = (monday - seoul_now).num_seconds();
    println!(
        "Weekend...resting until next KST Monday 9am: {} seconds",
        difference
    );
    sleep(Duration::from_secs(difference as u64)).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    println!("Connecting to mongo-db...");
    let mongo_db = std::env::var("MONGODB").expect("MONGODB must be set.");
    let client_options = ClientOptions::parse(mongo_db).await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let notice_collection = client.database("ajou").collection::<Notice>("notice");

    println!("Connected!");

    'main: loop {
        let seoul_now: DateTime<Tz> = Utc::now().with_timezone(&Seoul);

        match seoul_now.weekday() {
            Weekday::Sat | Weekday::Sun => {
                wait_until_weekday(seoul_now).await;
            }
            _ => {}
        }

        if seoul_now.hour() >= 19 || seoul_now.hour() <= 8 {
            wait_until_next_morning(seoul_now).await;
            continue 'main;
        }

        println!("Parsing notices now...");

        match update_database(&notice_collection).await {
            Ok(_) => {
                println!("Updated!, resting 30 mins...");
                sleep(Duration::from_secs(1800)).await;
            }
            Err(e) => {
                // eprintln!("Error: {}", e);
                println!("Encountered an {e}, resting for 5 mins...");
                sleep(Duration::from_secs(300)).await;
            }
        }
    }
}

#[tokio::test]
async fn test_parse_notice() {
    let notices = notice_parse("ajou", Some(7)).await.unwrap();

    println!("{:#?}", notices);
}
