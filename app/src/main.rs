#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
}

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

const STYLE: &str = include_str!("../assets/style.css");

#[component]
fn Home() -> Element {
    let mut row = use_signal(|| 0_u128);
    let mut col = use_signal(|| 0);

    let col_name = name_from_index(*col.read());

    let left = if let Some(next_col) = col.read().checked_sub(1) {
        rsx! {button { class: "left vcenter", onclick: move |_| col.set(next_col), "<" }}
    } else {
        rsx! {button { class: "left vcenter", disabled: true, "<" }}
    };
    let up = if let Some(next_row) = row.read().checked_sub(1) {
        rsx! {button { class: "top hcenter", onclick: move |_| row.set(next_row), "^" }}
    } else {
        rsx! {button { class: "top hcenter", disabled: true, "^" }}
    };
    rsx! {
        style { {STYLE} }
        div { class: "left top", "{col_name}{row}" }
        div { class: "right top" }
        div { class: "left bottom" }
        div { class: "right bottom" }
        div { class: "hcenter vcenter", id: "center", Grid {
            col: *col.read(),
            row: *row.read(),
        } }
        {left}
        button { class: "right vcenter", onclick: move |_| col += 1, ">" }
        {up}
        button { class: "bottom hcenter", onclick: move |_| row += 1, "v" }
    }
}

#[component]
fn Grid(col: u128, row: u128) -> Element {
    let style: String = (0..100)
        .map(|i| {
            let xpos = i * 100;
            let ypos = i * 20;
            let i = row + i;
            let j = col + i;
            format!(".row{i} {{ top: {ypos}px }} .col{j} {{ left: {xpos}px }}")
        })
        .collect();
    rsx! {
        style {
            {style}
        }
        for i in 0..100 {
            for j in 0..100 {
                Cell {
                    row: row + i,
                    col: col + j,
                }
            }
        }
    }
}
#[component]
fn Cell(col: u128, row: u128) -> Element {
    rsx! { input {
        class: "row{row} col{col} cell",
        value: "{name_from_index(col)}{row}",
    }}
}

fn name_from_index(mut col: u128) -> String {
    let mut s = String::new();
    loop {
        let rem = (col % 26) as u8;
        col /= 26;
        s.insert(0, (b'A' + rem).into());
        if col == 0 {
            break s;
        }
    }
}
