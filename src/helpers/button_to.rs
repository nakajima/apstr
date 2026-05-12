use maud::{Markup, html};
use seekwel_forms::FormMethod;

pub struct ButtonToOptions {
    method: FormMethod,
    confirm: Option<String>,
}
impl ButtonToOptions {
    pub fn with_method(mut self, method: FormMethod) -> Self {
        self.method = method;
        self
    }

    pub fn confirm(mut self, confirm: impl Into<String>) -> Self {
        self.confirm = Some(confirm.into());
        self
    }
}
impl Default for ButtonToOptions {
    fn default() -> Self {
        ButtonToOptions {
            method: FormMethod::Post,
            confirm: None,
        }
    }
}

pub fn button_to(label: &str, action: &str, options: ButtonToOptions) -> Markup {
    let onsubmit = if let Some(confirm) = options.confirm {
        format!("return confirm({:?})", confirm)
    } else {
        "".to_string()
    };

    html! {
      form action=(action) method=(options.method.form_method()) class="inline" onsubmit=(onsubmit) {
        @if let Some(method_override) = options.method.method_override() {
          input type="hidden" value=(method_override.to_uppercase()) name="_method";
        }
        button.link type="submit" { (label) };
      }
    }
}
