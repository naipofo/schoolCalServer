use chrono::{Datelike, Days, Local, NaiveTime, Weekday};
use ics::{
    properties::{DtEnd, DtStart, RRule, Summary},
    Event,
};
use num_traits::FromPrimitive;
use reqwest::{Client, Error};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

const VCAL_FMT: &str = "%Y%m%dT%H%M%S";
pub struct EduScraper<'a> {
    client: Client,
    subdomain: &'a str,
}

impl<'a> EduScraper<'a> {
    pub fn new(subdomain: &'a str) -> Self {
        Self {
            client: Client::builder().build().unwrap(),
            subdomain,
        }
    }

    pub async fn rcp_call<T>(&self, endpoint: &str, args: &str) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(self
            .client
            .post(format!(
                "https://{}.edupage.org/timetable/server/{}",
                self.subdomain, endpoint
            ))
            .body(format!("{{\"__args\":{args},\"__gsh\":\"00000000\"}}"))
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn cal_gen(&self, class_id: &str) -> Option<Vec<Event>> {
        let tt_args = format!(
            "[null,\"{}\"]",
            self.rcp_call::<serde_json::Value>("ttviewer.js?__func=getTTViewerData", "[null,2022]")
                .await
                .ok()?
                .get("r")?
                .get("regular")?
                .get("default_num")?
                .as_str()?
        );
        let data_responce = self
            .rcp_call::<Value>("regulartt.js?__func=regularttGetData", &tt_args)
            .await
            .ok()?;
        let tables = SlowTables(
            data_responce
                .get("r")?
                .get("dbiAccessorRes")?
                .get("tables")?
                .as_array()?,
        );

        let mut events = vec![];

        for card in tables.table("cards")? {
            let lesson = &tables.value("lessons", card.get("lessonid")?.as_str()?)?;
            if lesson
                .get("classids")?
                .as_array()?
                .iter()
                .any(|e| e.as_str().map(|e| e == class_id).unwrap_or(false))
            {
                let mut ev = Event::new(
                    Uuid::new_v4().to_string(),
                    Local::now().format(VCAL_FMT).to_string(),
                );
                ev.push(Summary::new(format!(
                    "{} {}",
                    tables
                        .value("subjects", lesson.get("subjectid")?.as_str()?)?
                        .get("name")?
                        .as_str()?,
                    tables
                        .value("classrooms", card.get("classroomids")?.get(0)?.as_str()?)
                        .and_then(|e| Some(e.get("short")?.as_str()?))
                        .unwrap_or("NONE")
                )));

                let period = tables.value("periods", card.get("period")?.as_str()?)?;
                let start_time =
                    NaiveTime::parse_from_str(period.get("starttime")?.as_str()?, "%H:%M").ok()?;
                let end_time =
                    NaiveTime::parse_from_str(period.get("endtime")?.as_str()?, "%H:%M").ok()?;

                let weekday = Weekday::from_u64(
                    card.get("days")?
                        .as_str()?
                        .chars()
                        .enumerate()
                        .find(|(_, c)| *c == '1')?
                        .0 as u64,
                )
                .unwrap();

                let current_date = Local::now().date_naive();
                let last_monday = current_date
                    - chrono::Duration::days(current_date.weekday().num_days_from_monday() as i64);

                let target_date = last_monday
                    + Days::new(
                        (weekday.num_days_from_monday()
                            - last_monday.weekday().num_days_from_monday())
                        .into(),
                    );

                ev.push(DtStart::new(format!(
                    "{}",
                    target_date.and_time(start_time).format(VCAL_FMT)
                )));
                ev.push(DtEnd::new(format!(
                    "{}",
                    target_date.and_time(end_time).format(VCAL_FMT)
                )));

                ev.push(RRule::new("FREQ=WEEKLY"));

                events.push(ev);
            }
        }

        Some(events)
    }
}

#[derive(Debug)]
struct SlowTables<'a>(&'a Vec<Value>);

impl SlowTables<'_> {
    fn table(&self, name: &str) -> Option<&Vec<Value>> {
        for value in self.0 {
            if let Value::String(id_str) = value.get("id")? {
                if id_str == name {
                    return Some(value.get("data_rows")?.as_array()?);
                }
            }
        }
        None
    }
    fn value(&self, table: &str, id: &str) -> Option<&Value> {
        for el in self.table(table)? {
            if el.get("id")?.as_str()? == id {
                return Some(el);
            }
        }
        None
    }
}
