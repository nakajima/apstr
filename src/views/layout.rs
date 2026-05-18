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
                nav.hstack.gap-8.mb-8 {
                    a href="/" {
                        h1 {
                            "APSTR "
                            span.white { (title) }
                        }
                    }
                }
                (body)

                script type="module" src="https://cdn.jsdelivr.net/npm/ionicons@latest/dist/ionicons/ionicons.esm.js" {}
                script src="https://cdn.jsdelivr.net/npm/ionicons@latest/dist/ionicons/ionicons.js" nomodule {}
            }
        }
    }
}
