use scraper::ElementRef;
use scraper::Selector;
use scraper::Html;

lazy_static! {
    static ref TITLE_SELECTOR: Selector = Selector::parse("title").unwrap();
    static ref BODY_SELECTOR: Selector = Selector::parse("body").unwrap();
}

fn traverse_node(node: ElementRef, field_texts: &mut Vec<(&str, String)>) {
    match node.value().name() {
        "h1"
        | "h2"
        | "h3"
        | "h4"
        | "h5"
        | "h6" => {
            field_texts.push(("heading", node.text().collect()));
        }
        _ => {
            if !node.has_children() {
                // field_texts.push(("body", node.text().collect()));
                return;
            }

            for child in node.children() {
                if let Some(el_child) = ElementRef::wrap(child) {
                    traverse_node(el_child, field_texts);
                } else {
                    if let Some(text) = child.value().as_text() {
                        if let Some(last) = field_texts.last_mut() {
                            if last.0 == "body" {
                                last.1 += text;
                            } else {
                                field_texts.push(("body", text.to_string()));
                            }
                        } else {
                            field_texts.push(("body", text.to_string()));
                        }
                    }
                }
            }
        }
    }
}

#[inline(always)]
pub fn get_html_field_texts(link: String, html_text: String) -> Vec<(&'static str, String)> {
    let mut field_texts: Vec<(&str, String)> = Vec::with_capacity(20);
    let document = Html::parse_document(&html_text);

    field_texts.push(("link", link));

    if let Some(title) = document.select(&TITLE_SELECTOR).next() {
        field_texts.push(("title", title.text().collect()));
    }

    if let Some(body) = document.select(&BODY_SELECTOR).next() {
        traverse_node(body, &mut field_texts);
    }

    field_texts
}
