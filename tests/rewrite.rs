mod tests {
    mod rewrite {
        use colored::Colorize;
        use std::path::Path;

        use similar::{ChangeTag, TextDiff};
        fn init() {
            let _ = env_logger::builder().is_test(true).try_init();
        }

        #[test]
        fn echo() {
            init();

            let schema = recad::Schema::load(Path::new("tests/echo/echo.kicad_sch")).unwrap();
            let mut file = std::fs::File::create("/tmp/summe.kicad_sch").unwrap();
            schema.write(&mut file).unwrap();

            let left = std::fs::read_to_string("tests/echo/echo.kicad_sch").unwrap();
            let right = std::fs::read_to_string("/tmp/summe.kicad_sch").unwrap();

            let diff = TextDiff::from_lines(
                left.as_str(),
                right.as_str(),
            );

            let mut diffs = 0;
            let mut last_offset = false;
            for change in diff.iter_all_changes() {
                if change.to_string().contains("(xy ") {
                    //println!("*{}", change.to_string().italic());
                    continue
                } else if change.to_string().contains("(offset ") {
                    //println!("*{}", change.to_string().italic());
                    last_offset = true;
                    continue
                } else if last_offset && change.to_string().trim() == ")" {
                    //println!("*{}", change.to_string().italic());
                    last_offset = false;
                    continue
                } else {
                    match change.tag() {
                        ChangeTag::Delete => { print!("-{}", change.to_string().red()); diffs+=1; },
                        ChangeTag::Insert => { print!("+{}", change.to_string().green()); diffs+=1;},
                        ChangeTag::Equal => { }, //print!(" {}", change); },
                    };
                }
            }
            assert_eq!(diffs, 0);
        }

        #[test]
        fn all_elements() {
            init();

            let schema = recad::Schema::load(Path::new("tests/all_elements/all_elements.kicad_sch")).unwrap();
            let mut file = std::fs::File::create("/tmp/all_elements.kicad_sch").unwrap();
            schema.write(&mut file).unwrap();

            let left = std::fs::read_to_string("tests/all_elements/all_elements.kicad_sch").unwrap();
            let right = std::fs::read_to_string("/tmp/all_elements.kicad_sch").unwrap();

            let diff = TextDiff::from_lines(
                left.as_str(),
                right.as_str(),
            );

            let mut diffs = 0;
            for change in diff.iter_all_changes() {
                if change.to_string().contains("(xy ") {
                    //println!("*{}", change.to_string().italic());
                } else if change.to_string().contains("(uuid ") {
                    //TODO only skip " and not the rest
                    //println!("*{}", change.to_string().italic());
                } else {
                    match change.tag() {
                        ChangeTag::Delete => { print!("-{}", change.to_string().red()); diffs+=1; },
                        ChangeTag::Insert => { print!("+{}", change.to_string().green()); diffs+=1;},
                        ChangeTag::Equal => { }, //print!(" {}", change); },
                    };
                }
            }
            assert_eq!(diffs, 16);
        }
    }
}

