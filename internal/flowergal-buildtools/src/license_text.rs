// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use cargo_about::licenses::{Gatherer, LicenseStore, LicenseInfo};
use cargo_about::licenses::config::{Config, KrateConfig, Ignore};
use spdx::{Licensee, LicenseId};
use spdx::expression::{ExprNode, Operator};
use hyphenation::Load;
use regex::Regex;

use build_const::ConstWriter;

//use crate::compression::{CompressibleAsset, do_lz77_compression};

const LICENSE_PREFERENCES: &[&str] = &["0BSD", "Unlicense", "MIT", "BSD-3-Clause", "Apache-2.0"];
const NO_INCLUSION_OBLIGATIONS: &[&str] = &["0BSD", "Unlicense"];

const WRAP_WIDTH: usize = 20;

/*
enum TextResource {
    Raw(String),
    Lz77(&'static [u32]),
}
impl CompressibleAsset for TextResource {
    fn compress(self) -> Result<Self, Box<dyn Error>> {
        match self {
            TextResource::Raw(s) => {
                Ok(TextResource::Lz77(do_lz77_compression(s.as_bytes(), true)?))
            }
            x => Ok(x),
        }
    }
}
*/

pub fn generate_text() -> Result<(), Box<dyn Error>> {
    let mut bc_out = ConstWriter::for_build("license_text_bc")?.finish_dependencies();

    let text = get_text()?;
    bc_out.add_value("LICENSE_TEXT", "&str", text);

    bc_out.finish();

    // TextResource::Raw(_text).compress()?;

    Ok(())
}

fn read_from_files(included_crate_licenses: BTreeMap<String, BTreeMap<&str, PathBuf>>) -> Result<BTreeMap<Vec<String>, String>, Box<dyn Error>> {
    // keyed on license text!
    let mut reverse_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (crate_name, licenses) in included_crate_licenses {
        let mut buf = String::new();
        for (_license_name, path) in licenses {
            File::open(path)?.read_to_string(&mut buf)?;
        }
        buf = buf.replace("–", "-");
        buf = buf.replace("©", "(c)");
        // FIXME: put some unicode chars in font, special-case the tile IDs here when we do
        buf = buf.replace("ń", "n");
        buf = buf.replace("ł", "t");
        let dirty_newline = Regex::new(r"(\r\n|\n\t|\n )")?;
        while dirty_newline.is_match(&buf) {
            buf = dirty_newline.replace_all(&buf, "\n").to_string();
        }
        let single_newline = Regex::new(r"([^\n])\n([^\n])")?;
        while single_newline.is_match(&buf) {
            buf = single_newline.replace_all(&buf, "$1 $2").to_string();
        }
        reverse_map.entry(buf).or_default().push(crate_name);
    }
    let forward_map = reverse_map.into_iter()
        .map(|(k, v)| (v, k))
        .collect();

    Ok(forward_map)
}

fn get_text() -> Result<String, Box<dyn Error>> {
    let included_crate_licenses = get_relevant_crate_licenses()?;

    let grouped_licenses = read_from_files(included_crate_licenses)?;

    let mut text = "\
    This demo uses the following crates \
    under their respective open-source licenses.\
    \n================\n\n".to_string();

    const WRAP_WIDTH_MINUS_4: usize = WRAP_WIDTH - 4;
    const WRAP_WIDTH_MINUS_3: usize = WRAP_WIDTH - 3;
    const WRAP_WIDTH_MINUS_2: usize = WRAP_WIDTH - 2;
    for (crate_names, license_text) in grouped_licenses {
        for crate_name in crate_names {
            match crate_name.len() {
                0 => return Err("crate had no name!".into()),
                1..=WRAP_WIDTH_MINUS_4 => writeln!(text, "= {} =", crate_name)?,
                WRAP_WIDTH_MINUS_3 | WRAP_WIDTH_MINUS_2 => writeln!(text, "={}=", crate_name)?,
                _ => writeln!(text, "{}\n===", crate_name)?,
            }
        }
        writeln!(text, "\n{}\n---\n", license_text)?;
    }

    let splitter = hyphenation::Standard::from_embedded(hyphenation::Language::EnglishUS)?;
    let options = textwrap::Options::new(WRAP_WIDTH).splitter(splitter);
    let wrapped = textwrap::fill(&text, options);

    Ok(wrapped)
}

fn get_relevant_crate_licenses() -> Result<BTreeMap<String, BTreeMap<&'static str, PathBuf>>, Box<dyn Error>> {
    // workaround for build tools and crates already accounted for in the cargo metadata (gba)
    // causing the root crate to have a confusing license detected
    // FIXME: we could just.. reorganize the workspace to not have the top-level be a crate?
    let mut crates: BTreeMap<String, _> = BTreeMap::new();
    crates.insert("flac-demo".to_string(), KrateConfig {
        additional: vec![],
        ignore: [
            ("BSD-3-Clause", "external/flac/COPYING.Xiph"),
            ("GFDL-1.2", "external/flac/COPYING.FDL"),
            ("GPL-2.0", "external/flac/COPYING.GPL"),
            ("LGPL-2.1", "external/flac/COPYING.LGPL"),
            ("Apache-2.0", "external/gba/LICENSE-APACHE2.txt"),
            ("MIT", "external/gba-compression/LICENSE"),
            ("GPL-3.0", "external/lossywav/gpl.txt"),
            ("BSD-3-Clause", "internal/flowergal-runtime/COPYING"),
            ("MIT", "internal/flowergal-runtime/COPYING-simpleflac"),
            ("AGPL-3.0", "internal/flowergal-buildtools/COPYING"),
        ].iter().map(|(lic, path)| Ignore {
            license: spdx::license_id(lic).unwrap(),
            license_file: path.into(),
            license_start: None,
            license_end: None
        }).collect()
    });

    let cfg = Config {
        targets: std::env::var("CARGO_BUILD_TARGET").into_iter().collect(),
        ignore_build_dependencies: true,
        ignore_dev_dependencies: true,
        accepted: LICENSE_PREFERENCES
            .iter()
            .map(|x| Licensee::parse(x).unwrap())
            .collect(),
        crates
    };
    let cargo_toml = std::env::current_dir()?.parent().unwrap().parent().unwrap().join("Cargo.toml");
    let krates = cargo_about::get_all_crates(
        cargo_toml,
        true,
        false,
        Vec::new(), // FIXME: can we determine what --features were passed to cargo build?
        &cfg
    )?;
    let gatherer = Gatherer::with_store(Arc::new(LicenseStore::from_cache()?));
    // threshold chosen to avoid autodetecting Makefiles in external/flac, etc.,
    //  actual license files are generally > 98.5% confidence
    let summary = gatherer.with_confidence_threshold(0.93).gather(&krates, &cfg);

    let mut included_crate_licenses = BTreeMap::new();
    for nfo in summary.nfos {
        let info = nfo.lic_info;
        let stack = reduce_license_expression_by_preference(&info);
        for file in nfo.license_files.iter()
            .filter(|lf| stack.contains(&lf.id)) {
                included_crate_licenses.entry(nfo.krate.name.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(file.id.full_name, file.path.clone());
        }
    }

    Ok(included_crate_licenses)
}

fn reduce_license_expression_by_preference(info: &LicenseInfo) -> Vec<LicenseId> {
    let mut stack = Vec::new();
    match &info {
        LicenseInfo::Expr(expr) => {
            let nodes: Vec<_> = expr.iter().collect();
            // FIXME: this logic extremely doesn't work for nodes.len() > 3
            assert!(nodes.len() <= 3);
            for node in nodes {
                match node {
                    ExprNode::Op(Operator::Or) => {
                        stack.sort_by_cached_key(|li| LICENSE_PREFERENCES
                            .iter()
                            .enumerate()
                            .find_map(|(index, name)| {
                                if *li == spdx::license_id(name).unwrap() {
                                    Some(index)
                                } else {
                                    None
                                }
                            })
                        );
                        stack.pop();
                    }
                    ExprNode::Op(Operator::And) => {}
                    ExprNode::Req(req) => {
                        stack.push(req.req.license.id().unwrap());
                    }
                }
            }
        }
        LicenseInfo::Unknown => {}
    }
    stack.drain_filter(|li| NO_INCLUSION_OBLIGATIONS.iter()
        .map(|name| spdx::license_id(name).unwrap())
        .find(|x| x == li)
        .is_some());
    stack
}
