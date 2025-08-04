use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

#[derive(PartialEq, PartialOrd)]
struct Pair(String, String);

impl Pair {
    fn new(a: String, b: String) -> Pair {
        if a > b {
            Pair(b, a)
        } else {
            Pair(a, b)
        }
    }

    fn to_string(&self) -> String {
        let Pair(a, b) = self;
        format!("\"{}\" -- \"{}\"", a, b)
    }
}

fn make_pairs(set: &BTreeMap<String, i32>) -> Vec<String> {
    let vals: Vec<_> = set.iter().map(|(k, _)| k).collect();
    let mut res = Vec::new();

    for i in 0..vals.len() {
        for j in (i + 1)..vals.len() {
            res.push(Pair::new(vals[i].clone(), vals[j].clone()).to_string());
        }
    }

    res
}

type FileName = String;
type Author = String;
type Changes = i32;
type AuthorPair = String;

fn save_state(
    files: &BTreeMap<FileName, BTreeMap<Author, Changes>>, 
    authors: &BTreeSet<Author>,
    file_to_id: &BTreeMap<&str, u32>,
    changemap: &BTreeMap<(u32, u32), u32>,
)
    -> Result<(), std::io::Error>

{
    use std::io::Write;

    let author_to_id: BTreeMap<&str, usize>  = authors.iter()
        .enumerate().map(|(idx, ss)| (ss.as_str(), idx)).collect();

    let file = std::fs::File::create(".colabnet")?;
    let mut out = std::io::BufWriter::new(file);

    for (author, _idx) in author_to_id.iter() {
        writeln!(out, "{}", author)?;
    }

    writeln!(out, "")?;

    for (file, _idx) in file_to_id.iter() {
        writeln!(out, "{}", file)?;
    }

    writeln!(out, "")?;

    for (file, changemap) in files.iter() {
        for (author, changes) in changemap.iter() {
            writeln!(out, "{} {} {}", 
                file_to_id.get(file as &str).unwrap(),
                author_to_id.get(author as &str).unwrap(),
                changes)?;
        }
    }

    writeln!(out, "")?;

    for ((i, j), count) in changemap.iter() {
        if i <= j {
            writeln!(out, "{i} {j} {count}")?;
        }
    }

    Ok(())
}

struct ColabNetDatabase {
    authors: Vec<String>,
    files: Vec<String>,
    files_to_authors: BTreeMap<u32, BTreeMap<u32, u32>>,
    files_to_files: BTreeMap<u32, BTreeMap<u32, u32>>, // file -> (file, changes)
}

impl ColabNetDatabase {
    pub fn find_file(&self, fname: &str) -> Option<u32> {
        if let Ok(res) = self.files.binary_search(&fname.to_string()) {
            Some(res as u32)
        } else {
            None
        }
    }
    pub fn find_author(&self, author: &str) -> Option<u32> {
        if let Ok(res) = self.authors.binary_search(&author.to_string()) {
            Some(res as u32)
        } else {
            None
        }
    }
    pub fn get_author(&self, idx: u32) -> Option<&str> {
        if (idx as usize) < self.authors.len() {
            Some(&self.authors[idx as usize])
        } else {
            None
        }
    }
    pub fn get_file(&self, idx: u32) -> Option<&str> {
        if (idx as usize) < self.files.len() {
            Some(&self.files[idx as usize])
        } else {
            None
        }
    }
    pub fn authors_of_file(&self, fname: &str) -> Option<Vec<(&str, u32)>> {
        self.find_file(fname)
            .map(|file_idx| {
                self.files_to_authors
                    .get(&file_idx).unwrap()
                    .iter()
                    .map(|(authno, chg)| (self.get_author(*authno).unwrap(), *chg))
                    .collect()
            })
    }
    pub fn files_correlated(&self, fname: &str) -> Option<Vec<(&str, u32)>> {
        if let Some(fileno) = self.find_file(fname) {
            if let Some(otherfiles) = self.files_to_files.get(&fileno) {
                let temp: Vec<(&str, u32)> = otherfiles.iter()
                    .map(|(fidx, changes)| (self.get_file(*fidx).unwrap(), *changes))
                    .collect();

                return Some(temp);
            }
        }

        None
    }
    pub fn from_disk(load_file_to_file: bool) -> Result<Self, std::io::Error> {
        use std::io::BufRead;

        let file = std::fs::File::open(".colabnet")?;
        let fin = std::io::BufReader::new(file);

        let mut mode = 0; // 0 - authors, 1 - files, 2 - mappings, 3 - file2file
        let mut authors = Vec::new();
        let mut files = Vec::new();
        let mut files_to_authors: BTreeMap<u32, BTreeMap<u32, u32>> = BTreeMap::new();
        let mut files_to_files: BTreeMap<u32, BTreeMap<u32, u32>> = BTreeMap::new();

        for line in fin.lines().filter_map(|l| l.ok()) {

            if line == "" {
                mode += 1;
            }

            if mode == 0 {
                authors.push(line.trim().to_string());
            } else if mode == 1 {
                files.push(line.trim().to_string());
            } else if mode == 2 {
                let nums: Vec<_> = line.split_whitespace().collect();

                if nums.len() == 0 {
                    continue;
                }

                let fileno = nums[0].parse::<u32>().expect("failed to parse num");
                let authorno = nums[1].parse::<u32>().expect("failed to parse num");
                let changesno = nums[2].parse::<u32>().expect("failed to parse num");
                let f = files_to_authors.entry(fileno)
                    .or_insert(BTreeMap::new());

                let changes = f.entry(authorno)
                    .or_insert(0);
                *changes = changesno as u32;
            } else if mode == 3 {
                if !load_file_to_file {
                    break;
                }
                let nums: Vec<_> = line.split_ascii_whitespace().collect();

                if nums.len() == 3 {
                    let f1 = nums[0].parse();
                    let f2 = nums[1].parse();
                    let changes = nums[2].parse();

                    if let (Ok(f1), Ok(f2), Ok(changes)) = (f1, f2, changes) {
                        let entry = files_to_files.entry(f1).or_default();
                        let e = entry.entry(f2).or_default();
                        *e = changes;

                        if f1 != f2 {
                            let entry = files_to_files.entry(f2).or_default();
                            let e = entry.entry(f1).or_default();
                            *e = changes;
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok(ColabNetDatabase { files, authors, files_to_authors, files_to_files })
    }
}

#[test]
fn test_normalize_fname() {
    let nname = normalize_fname("/hello/{test => test1}/world.txt");
    assert_eq!(nname, "/hello/test1/world.txt");
    let nname = normalize_fname("test => test1");
    assert_eq!(nname, "test1");
}

fn normalize_fname(fname: &str) -> String {
    if fname.contains("=>") {
        if fname.contains('{') {
            let a: Vec<_> = fname.split('{').collect();
            let b: Vec<_> = a[1].split('}').collect();
            let c: Vec<_> = b[0].split("=>").collect();
            a[0].to_string() + c[1].trim() + b[1]
        } else {
            let c: Vec<_> = fname.split("=>").collect();
            c[1].trim().to_string()
        }
    } else {
        fname.to_string()
    }
}

fn main() -> Result<(), std::io::Error> {
    let sargs: Vec<_> = std::env::args().collect();
    use std::io::Write;

    if sargs[1] == "owners" {
        let now = std::time::Instant::now();
        let database = ColabNetDatabase::from_disk(false)?;
        eprintln!("load_state new {:?}", now.elapsed());
        
        if let Some(mut authors) = database.authors_of_file(&sargs[2]) {
            authors.sort_by_key(|(_a, ch)| *ch);

            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "CHANGES\tAUTHOR")?;
            writeln!(stdout, "=======================")?;
            for r in authors.iter().rev() {
                writeln!(stdout, "{}\t{}", r.1, r.0)?;
            }
        }

        return Ok(());
    } else if sargs[1] == "cousins" {
        let now = std::time::Instant::now();
        let database = ColabNetDatabase::from_disk(true)?;
        eprintln!("load_state new {:?}", now.elapsed());
        
        if let Some(mut res) = database.files_correlated(&sargs[2]) {
            if res.is_empty() {
                return Ok(());
            }
            res.sort_by_key(|(_a, ch)| *ch);
            let total_changes = res[res.len()-1].1 as f64;

            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "TOTAL CHANGES: {total_changes}")?;
            writeln!(stdout, "%\tFILE")?;
            writeln!(stdout, "=======================")?;
            for r in res.iter().rev() {
                writeln!(stdout, "{:>6.2}\t{}", 100.0 * r.1 as f64 / total_changes, r.0)?;
            }
        }
        return Ok(());
    }


    let mut args = vec![
        "--no-pager",
        "log",
        "--stat",
        "--stat-width=1000",
        "--stat-name-width=800",
    ];

    for i in 1..sargs.len() {
        if sargs[i] == "--" {
            break;
        }
        args.push(&sargs[i]);
    }

    let graph_mode = sargs.iter().find(|s| *s == "graph").is_some();
    let scan_mode = sargs.iter().find(|s| *s == "scan").is_some();

    let git_log = Command::new("git").args(args).output()?;
    let statline = Regex::new("^ [^ ].*\\|").unwrap();

    let out = String::from_utf8_lossy(&git_log.stdout);

    let mut author = String::new();
    let mut authors = BTreeSet::new();

    // Files and their authors
    let mut files: BTreeMap<FileName, BTreeMap<Author, Changes>> = BTreeMap::new();
    // Pair -> Collection of files
    let mut graph: BTreeMap<AuthorPair, BTreeSet<FileName>> = BTreeMap::new();

    for line in out.lines() {
        if line.starts_with("Author:") {
            if let Some(aut) = line.split(':').nth(1) {
                author = aut.trim().to_string();
                authors.insert(author.clone());
            }
        } else if statline.is_match(line) {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 2 {
                let fname = normalize_fname(parts[0].trim());
                let f = files.entry(fname).or_insert(BTreeMap::new());
                let ch = parts[1]
                    .split_whitespace()
                    .next()
                    .map(|s| s.parse::<i32>().ok())
                    .flatten()
                    .unwrap_or(0);


                let changes = f.entry(author.clone()).or_insert(0);
                *changes = *changes + ch;
            }
        }
    }

    let file_to_id: BTreeMap<&str, u32> = files.keys()
        .enumerate().map(|(idx, ss)| (ss.as_str(), idx as u32)).collect();

    let mut changemap: BTreeMap<(u32, u32), u32> = Default::default();
    let mut scratch = Vec::new();

    for line in out.lines() {
        if line.starts_with("commit") {
            for i in 0..scratch.len() {
                for j in i..scratch.len() {
                    let entry = changemap.entry((scratch[i], scratch[j])).or_default();
                    *entry += 1;
                    if i != j {
                        let entry = changemap.entry((scratch[j], scratch[i])).or_default();
                        *entry += 1;
                    }
                }
            }

            scratch.clear();
        }

        if statline.is_match(line) {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 2 {
                let fname = normalize_fname(parts[0].trim());
                scratch.push(*file_to_id.get(&fname as &str).unwrap());
            }
        }
    }

    save_state(&files, &authors, &file_to_id, &changemap).expect("failed saving state");
    
    if scan_mode {
        return Ok(())
    }

    if graph_mode {
        for (file, fauthors) in files {
            for pair in make_pairs(&fauthors) {
                let files = graph.entry(pair).or_insert(BTreeSet::new());
                files.insert(file.clone());
            }
        }

        println!("graph colabnet {{");
        for (k, v) in graph {
            println!(
                "{} [weight={}, penwidth={}] // {:?}",
                k,
                v.len(),
                v.len(),
                v
            );
        }

        println!("}}");
    } else {
        // file mode
        println!("digraph colabnet {{");
        for (file, authors) in files.iter() {
            for (author, w) in authors.iter() {
                println!("\"{author}\" -> \"{file}\" [weight={w}] ")
            }
        }
        println!("}}");
    }

    Ok(())
}
