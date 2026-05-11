use maud::{DOCTYPE, Markup, html};

pub fn page(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title { (title) }
                link rel="stylesheet" href="/assets/MyProportional.css";
                // link rel="stylesheet" href="/assets/normalize.css";
                link rel="stylesheet" href="/assets/style.css";
            }
            body {
                (body)
            }
        }
    }
}
