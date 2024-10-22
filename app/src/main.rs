#![allow(non_snake_case)]

use cell_index::{Col, Row};
use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

mod cell_index;

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
    let mut row = use_signal(|| Row::FIRST);
    let mut col = use_signal(|| Col::FIRST);

    let left = if let Some(next_col) = *col.read() - 1 {
        rsx! {button { class: "left vcenter", onclick: move |_| col.set(next_col), "<" }}
    } else {
        rsx! {button { class: "left vcenter", disabled: true, "<" }}
    };
    let up = if let Some(next_row) = *row.read() - 1 {
        rsx! {button { class: "top hcenter", onclick: move |_| row.set(next_row), "^" }}
    } else {
        rsx! {button { class: "top hcenter", disabled: true, "^" }}
    };
    let right = if let Some(next_col) = *col.read() + 1 {
        rsx! {button { class: "right vcenter", onclick: move |_| col.set(next_col), ">" }}
    } else {
        rsx! {button { class: "right vcenter", disabled: true, "<" }}
    };
    let down = if let Some(next_row) = *row.read() + 1 {
        rsx! {button { class: "bottom hcenter", onclick: move |_| row.set(next_row), "v" }}
    } else {
        rsx! {button { class: "bottom hcenter", disabled: true, "v" }}
    };
    rsx! {
        style { {STYLE} }
        div { class: "left top", "{col}{row}" }
        div { class: "right top" }
        div { class: "left bottom" }
        div { class: "right bottom" }
        div { class: "hcenter vcenter", id: "center", Grid {
            col: *col.read(),
            row: *row.read(),
        } }
        {left} {up} {right} {down}
    }
}

const DISPLAY_ROWS: u128 = 30;
const DISPLAY_COLS: u128 = 10;

#[component]
fn Grid(col: Col, row: Row) -> Element {
    let style: String = (0..DISPLAY_COLS)
        .map(|i| {
            let xpos = i * 100;
            let j = (col + i).unwrap();
            format!(".col{j} {{ left: {xpos}px }}")
        })
        .chain((0..DISPLAY_ROWS).map(|i| {
            let ypos = i * 20;
            let i = (row + i).unwrap();
            format!(".row{i} {{ top: {ypos}px }}")
        }))
        .collect();
    rsx! {
        style {
            {style}
        }
        for i in 0..DISPLAY_ROWS {
            for j in 0..DISPLAY_COLS {
                Cell {
                    row: (row + i).unwrap(),
                    col: (col + j).unwrap(),
                }
            }
        }
    }
}
#[component]
fn Cell(col: Col, row: Row) -> Element {
    rsx! { input {
        class: "row{row} col{col} cell",
        value: "{col}{row}",
    }}
}
