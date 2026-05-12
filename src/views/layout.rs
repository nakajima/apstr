use maud::{DOCTYPE, Markup, html};

pub fn page(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title { (title) }
                link rel="stylesheet" href="/assets/CommitMono.css";
                link rel="stylesheet" href="/assets/normalize.css";
                link rel="stylesheet" href="/assets/style.css";
            }
            body {
                nav.hstack.gap-8 {
                    a href="/" {
                        h1 { "APSTR" }
                    }
                    a href="/apps/new" {
                        "add app"
                    }
                    a href="/status" {
                        "status"
                    }

                }
                (body)
            }
        }
    }
}
