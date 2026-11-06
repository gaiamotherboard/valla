//! The one page: a form to add items, a search box, and the list.

use crate::store::{Item, STATUSES};

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn status_select(name: &str, current: &str) -> String {
    let opts: String = STATUSES
        .iter()
        .map(|s| format!("<option{}>{}</option>", if *s == current { " selected" } else { "" }, s))
        .collect();
    format!("<select name=\"{}\">{}</select>", name, opts)
}

pub fn render(title: &str, items: &[&Item], q: &str, total: usize, notice: &str) -> String {
    let rows: String = items
        .iter()
        .map(|i| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><form method=\"post\" action=\"/status\" class=\"inline\">\
                 <input type=\"hidden\" name=\"id\" value=\"{}\">{}<button>Set</button></form></td></tr>",
                i.id,
                esc(&i.name),
                esc(&i.location),
                i.id,
                status_select("status", &i.status)
            )
        })
        .collect();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\">\
<title>{t}</title><style>
body{{font-family:system-ui,sans-serif;max-width:900px;margin:2rem auto;padding:0 1rem;color:#222}}
h1{{font-weight:600}} form{{margin:.5rem 0}} .inline{{display:inline;margin:0}}
input,select,button{{font-size:1rem;padding:.35rem}} table{{border-collapse:collapse;width:100%;margin-top:1rem}}
td,th{{border-bottom:1px solid #ddd;padding:.4rem;text-align:left}} .notice{{color:#a33}}
</style></head><body>
<h1>{t}</h1>
<p class=\"notice\">{n}</p>
<form method=\"post\" action=\"/add\">
<input name=\"name\" placeholder=\"Item\" required> <input name=\"location\" placeholder=\"Location\"> {sel} <button>Add</button>
</form>
<form method=\"get\" action=\"/\"><input name=\"q\" value=\"{q}\" placeholder=\"Search\"> <button>Search</button></form>
<p>{shown} of {total} items</p>
<table><tr><th>#</th><th>Item</th><th>Location</th><th>Status</th></tr>{rows}</table>
</body></html>",
        t = esc(title),
        n = esc(notice),
        sel = status_select("status", STATUSES[0]),
        q = esc(q),
        shown = items.len(),
        total = total,
        rows = rows
    )
}
