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
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|&id| id < r.next_id));
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
            title: Some("my show".into())
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

#[test]
fn the_same_file_through_a_symlinked_folder_imports_once() {
    let dir = temp_dir("symlink");
    std::fs::create_dir_all(dir.join("real")).unwrap();
    std::fs::write(dir.join("real/show.md"), SHOW).unwrap();
    std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).unwrap();
    let library = Library::new(dir.join("Rundowns"));
    let (a, _) = library.open_markdown(&dir.join("real/show.md")).unwrap();
    let (b, _) = library
        .open_markdown(&dir.join("link/../real/show.md"))
        .unwrap();
    let (c, _) = library.open_markdown(&dir.join("link/show.md")).unwrap();
    assert_eq!((&a, &a), (&b, &c));
}

#[test]
fn an_unreadable_rundown_stays_listed_and_loading_it_says_why() {
    let dir = temp_dir("corrupt");
    let library = Library::new(dir.join("Rundowns"));
    library.save("1", &Rundown::import(SHOW, "good")).unwrap();
    std::fs::write(dir.join("Rundowns/2.json"), "{ not json").unwrap();
    let mut titles: Vec<_> = library.list().into_iter().map(|e| e.title).collect();
    titles.sort();
    assert_eq!(titles, [None, Some("good".to_string())]);
    assert!(library.load("2").is_err());
}

#[test]
fn a_rundown_from_a_newer_format_is_refused() {
    let dir = temp_dir("newer");
    let library = Library::new(dir.join("Rundowns"));
    let mut r = Rundown::import(SHOW, "future");
    r.format = 99;
    library.save("1", &r).unwrap();
    assert!(library.load("1").is_err());
}

#[test]
fn a_rundown_saved_before_next_id_existed_never_reuses_a_card_id() {
    let dir = temp_dir("legacy");
    let library = Library::new(dir.join("Rundowns"));
    library.save("1", &Rundown::import(SHOW, "old")).unwrap();
    let path = dir.join("Rundowns/1.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("next_id");
    std::fs::write(&path, json.to_string()).unwrap();
    assert_eq!(library.load("1").unwrap().next_id, 5);
}
