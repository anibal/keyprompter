use std::path::PathBuf;

use keyprompter::{
    library::{Entry, Library},
    rundown::Rundown,
};

const SHOW: &str = "intro text is dropped\n## Cold open\nfirst line\n\n# Part A\n## One\n- a **bold** bullet\nA paragraph\nthat continues.\n### Aside\n## Two\n# Part B\n## Three\n";

fn cues(r: &Rundown) -> Vec<(Option<&str>, &str, &str)> {
    r.sections
        .iter()
        .flat_map(|s| {
            s.cards
                .iter()
                .map(|c| (s.title.as_deref(), c.cue.as_str(), c.body.as_str()))
        })
        .collect()
}

#[test]
fn import_keeps_sections_cues_and_raw_bodies() {
    let r = Rundown::import(SHOW, "show");
    assert_eq!(
        cues(&r),
        [
            (None, "Cold open", "first line"),
            (
                Some("Part A"),
                "One",
                "- a **bold** bullet\nA paragraph\nthat continues.\n### Aside"
            ),
            (Some("Part A"), "Two", ""),
            (Some("Part B"), "Three", ""),
        ]
    );
}

#[test]
fn export_then_import_gives_back_the_same_rundown() {
    let r = Rundown::import(SHOW, "show");
    assert_eq!(Rundown::import(&r.export(), "show"), r);
}

#[test]
fn every_card_gets_its_own_id() {
    let r = Rundown::import(SHOW, "show");
    let mut ids: Vec<u64> = r
        .sections
        .iter()
        .flat_map(|s| s.cards.iter().map(|c| c.id.0))
        .collect();
    ids.dedup();
    assert_eq!(ids.len(), 4);
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kp-lib-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_saved_rundown_loads_back_unchanged_and_leaves_no_temp_file() {
    let dir = temp_dir("save");
    let library = Library::new(dir.join("Rundowns"));
    let r = Rundown::import(SHOW, "show");
    library.save("1", &r).unwrap();
    assert_eq!(library.load("1").unwrap(), r);
    let files: Vec<_> = std::fs::read_dir(dir.join("Rundowns"))
        .unwrap()
        .map(|f| f.unwrap().file_name())
        .collect();
    assert_eq!(files, ["1.json"]);
}

#[test]
fn opening_the_same_markdown_twice_imports_it_once() {
    let dir = temp_dir("open");
    let md = dir.join("my show.md");
    std::fs::write(&md, SHOW).unwrap();
    let library = Library::new(dir.join("Rundowns"));
    let (first, imported) = library.open_markdown(&md).unwrap();
    assert_eq!(imported.title, "my show");
    let (again, _) = library.open_markdown(&md).unwrap();
    assert_eq!(again, first);
    assert_eq!(
        library.list(),
        [Entry {
            id: first,
            title: "my show".into()
        }]
    );
}

#[test]
fn a_missing_markdown_file_is_an_error_and_imports_nothing() {
    let dir = temp_dir("missing");
    let library = Library::new(dir.join("Rundowns"));
    assert!(library.open_markdown(&dir.join("nope.md")).is_err());
    assert!(library.list().is_empty());
}
