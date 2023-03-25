use ics::ICalendar;
use rocket::{get, launch, routes, State};
use serde::{Deserialize, Serialize};

pub mod calgen;

#[derive(Serialize, Deserialize)]
struct AppData {
    auth: AuthData,
    config: ConfigData,
}

#[derive(Serialize, Deserialize)]
struct AuthData {
    lib_access_token: String,
}

#[derive(Serialize, Deserialize)]
struct ConfigData {
    edu_subdomain: String,
    edu_class_id: String,

    lib_secret: String,
}

#[get("/ical")]
async fn hello(app: &State<AppData>) -> String {
    if let Some(ev) = calgen::EduScraper::new(&app.config.edu_subdomain)
        .cal_gen(&app.config.edu_class_id)
        .await
    {
        let mut calendar =
            ICalendar::new("2.0", "-//xyz Corp//NONSGML PDA Calendar Version 1.0//EN");
        for e in ev {
            calendar.add_event(e);
        }
        calendar.to_string()
    } else {
        "Error".to_string()
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(AppData {
            auth: AuthData {
                lib_access_token: "".to_string(),
            },
            config: ConfigData {
                edu_subdomain: "".to_string(),
                edu_class_id: "".to_string(),
                lib_secret: "".to_string(),
            },
        })
        .mount("/", routes![hello])
}
