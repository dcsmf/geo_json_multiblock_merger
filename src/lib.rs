use anyhow::{anyhow, Context};
use geojson::{FeatureCollection, Geometry, JsonValue, PolygonType, Value};
use rust_i18n::t;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

rust_i18n::i18n!("locales");

pub fn run(parent_path: &Path) -> anyhow::Result<()> {
    let con = inquire::Confirm::new(t!("msg.continue_or_not").as_ref())
        .with_default(true)
        .prompt();
    if let Err(e) = con {
        return Err(e.into());
    } else if !con.unwrap_or(false) {
        return Err(anyhow!(t!("err.user_cancel")));
    }
    let select = rfd::FileDialog::new().pick_folder();
    if select.is_none() {
        return Err(anyhow!(t!("err.user_dont_select")));
    }
    let paths = find_files_recursion(&select.unwrap()).context("寻找文件出错")?;
    if paths.is_empty() {
        println!("{}", t!("msg.no_file_found"));
    } else {
        for path in paths {
            if let Some(file_name) = path.file_name() {
                println!("{} {:?}", t!("msg.read_file"), file_name);
                match process_file(path.clone()) {
                    Ok(geo_json) => {
                        if let Err(e) = write_file(parent_path, file_name, geo_json) {
                            eprintln!("{:?}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "{}",
                            t!("err.process_fail", path = path.to_string_lossy(), e = e)
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn process_file(path_buf: PathBuf) -> anyhow::Result<FeatureCollection> {
    // 从文件里读取geojson数据
    let file = File::open(path_buf)?;
    let reader = BufReader::new(file);
    let value: serde_json::Map<String, JsonValue> = serde_json::from_reader(reader)?;
    // 认为geojson是FeatureCollection
    let mut geo_json =
        geojson::FeatureCollection::try_from(value).context(t!("err.not_geojson"))?;
    // 遍历FeatureCollection的Feature数据
    // 可变借用features，因为要修改里面的数据
    for feature in &mut geo_json.features {
        // 读取properties对象里name字段，输出表示正在处理
        if let Some(properties) = &feature.properties {
            if let Some(name) = properties.get("name") {
                if let Some(name) = name.as_str() {
                    println!("{}", t!("msg.process_feature", name = name))
                }
            }
        }
        // 借用geometry
        if let Some(geometry) = &feature.geometry {
            // 如果geometry的type是GeometryCollection，则准备提取里面的Polygon出来外层
            match geometry.value {
                Value::GeometryCollection(ref geometries) => {
                    if let Ok(geometry) = process_geometries(geometries) {
                        if geometry.is_some() {
                            feature.geometry = geometry;
                        }
                    }
                }
                _ => continue,
            }
        }
    }
    Ok(geo_json)
}

/// 核心的处理函数，返回修改好的geometry
fn process_geometries(geometries: &Vec<Geometry>) -> anyhow::Result<Option<Geometry>> {
    // 把嵌套的Polygon单独拿出来，其他类型不用处理
    let mut polygons: Vec<PolygonType> = Vec::new();

    for geometry in geometries {
        if let Value::Polygon(ref polygon_type) = geometry.value {
            polygons.push(polygon_type.clone());
        }
    }

    if !polygons.is_empty() {
        return Ok(Some(Geometry::new(Value::MultiPolygon(polygons))));
    }

    Ok(None)
}

fn write_file(
    parent_path: &Path,
    file_name: &OsStr,
    geo_json: FeatureCollection,
) -> anyhow::Result<()> {
    let path = parent_path.join("gmm_out");
    if !path.exists() {
        fs::create_dir_all(&path).context(t!("err.create_folder_fail"))?;
    }
    let path = path.join(file_name);
    match serde_json::to_string_pretty(&geo_json) {
        Ok(json) => match File::create(path) {
            Ok(mut file) => match file.write_all(json.as_bytes()) {
                Ok(_) => {
                    println!("{}\n", t!("msg.process_complete"));
                    Ok(())
                }
                Err(e) => Err(anyhow!(t!("err.write_file_fail", e = e))),
            },
            Err(e) => Err(anyhow!(t!("err.create_file_fail", e = e))),
        },
        Err(e) => Err(anyhow!(t!("err.trans_json_fail", e = e))),
    }
}

/// 在当前目录寻找geojson文件
fn find_files_recursion(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files_path = Vec::new();

    // 读取目录内容
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && (path.extension().and_then(OsStr::to_str) == Some("json")
                || path.extension().and_then(OsStr::to_str) == Some("geojson"))
        {
            files_path.push(path);
        }
    }

    Ok(files_path)
}
