use std::fs;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;

use csv::WriterBuilder;
use walkdir::WalkDir;  // ディレクトリとファイルを再起的に操作するためのクレート
use anyhow::{Result, Context, anyhow};  // エラー処理をいい感じにするクレート

mod get_input; // get_input.rsのモジュールをインポート
use crate::get_input::{get_expected_input, get_expected_output}; 


// 結果をCSVに書き込むための構造体
#[derive(Debug, serde::Serialize)]
struct TestResult {
    file_name: String,
    status: String,
    error_details: String,
}

fn main() -> Result<()> {
    // 実行ディレクトリの直下に現在時刻のファイル名のCSVを作成
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let csv_path = timestamp + ".csv";
    let mut csv_file = WriterBuilder::new()
        .has_headers(true)
        .from_path(&csv_path)
        .context(format!("CSVファイル作成に失敗しました: {}", csv_path))?;
    

    // 標準入力からpythonのフォルダパスをうけとる
    println!("Pythonファイルが含まれるフォルダパスを入力してください:");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let folder_path = input.trim();
    // 現在のディレクトリ直下にそのフォルダがあるか確認
    let folder_path = Path::new(folder_path);
    if !folder_path.exists() || !folder_path.is_dir() {
        return Err(anyhow!("指定されたパスは有効なディレクトリではありません: {:?}", folder_path));
    }


    // 期待される標準入出力を入れておくためのmap
    let mut input_contents: HashMap<String, String> = HashMap::new();
    let mut output_contents: HashMap<String, Vec<String>> = HashMap::new();


    // 標準入力のファイルのあるフォルダもしくは1つのテキストファイルを受け取る
    println!("標準入力のファイルのあるフォルダもしくは1つのテキストファイル(ex : input.txt)を入力してください:");
    let mut input_file = String::new();
    io::stdin().read_line(&mut input_file)?;
    let input_file = Path::new(input_file.trim());
    get_expected_input(&mut input_contents, input_file).context("インプットファイルの読み込みに失敗しました")?;


    println!("想定される出力のファイルのあるフォルダもしくは1つのテキストファイル(ex : output.txt)を入力してください");
    let mut output_file = String::new();
    io::stdin().read_line(&mut output_file)?;
    let output_file = Path::new(output_file.trim());
    get_expected_output(&mut output_contents, output_file).context("アウトプットファイルの読み込みに失敗しました")?;

    
    println!("フォルダ内のPythonファイルをテストします: {:?}", folder_path);


    
    // フォルダ内のすべてのPythonファイルを走査
    for entry in WalkDir::new(folder_path) // 与えられたパスをルートとして新しいdirectory walkerを作成
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir() && e.path().extension().map_or(false, |ext| ext == "py")) // ファイルに拡張子がない場合.map_or()のデフォルト値falseが適用される
    {
        let python_file = entry.path();
        let file_name = python_file.file_stem().unwrap().to_string_lossy().to_string(); // 拡張子を除いたファイル名のみを取得
        
        println!("テスト実行中: {}", file_name);
        
        
        match test_python_file(python_file, &input_contents, &output_contents) {
            Ok((status, error_details)) => {
                println!("  結果: {}", status);
                if !error_details.is_empty() {
                    println!("  エラー詳細: {}", error_details);
                }

                csv_file.serialize(TestResult { // 構造体からをCSVファイルにシリアライズ
                    file_name,
                    status,
                    error_details,
                })?;
            },
            Err(e) => {
                csv_file.serialize(TestResult {
                    file_name,
                    status: "ERROR".to_string(),
                    error_details: format!("実行エラー: {}", e),
                })?;
                
                println!("  実行エラー: {}", e);
            }
        }
    }
    
    csv_file.flush()?;
    println!("テスト完了。結果は {} に保存されました", csv_path);
    
    Ok(())
}



// 処理自体（この関数の実行自体）が成功したら、
// 期待される出力と実行結果を比較して、一致か不一致かのstatusを返す
// さらに不一致の場合はその詳細について返す
fn test_python_file(file_path: &Path, input: &HashMap<String, String>, expected_output: &HashMap<String, Vec<String>>) -> Result<(String, String),  anyhow::Error> {
    
    let mut result = true;
    let mut detail = String::new();

    for (i, input) in input.iter() {
        // Python実行コマンドを作成
        let mut command = Command::new("python");
        command
            .arg(file_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().context("Pythonプロセスの実行に失敗しました")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).context("標準入力の書き込みに失敗しました")?;
        }
        
        // プロセスの終了を待ち標準出力と標準エラーを取得
        let output = child.wait_with_output().context("Pythonプロセスの実行結果の取得に失敗しました")?;
        
        // 標準出力と標準エラーを取得
        let stdout: String = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr: String = String::from_utf8_lossy(&output.stderr).trim().to_string();

        let stdout:Vec<String> = stdout.lines().map(|line| line.to_string()).collect::<Vec<String>>();
        let expected_stdout: &Vec<String> = expected_output.get(i).ok_or_else(|| anyhow!("期待される標準出力例が見つかりません"))?;
        
        // 実行結果と期待される出力を比較
        // ここで実行数を変えることでcheckのゆるさや厳しさを調整するようにする
        (result, detail) = complete_match(stdout, &expected_stdout);
    }

    println!("  実行結果: {}, 詳細 {}", result, detail);

    if result {
        return Ok(("PASS".to_string(), "".to_string()));
    } else {
        return Ok(("FAIL".to_string(), detail));
    }
}


// 完全一致かどうかを判定する
fn complete_match(stdout:Vec<String>, expected_stdout:&Vec<String>) -> (bool, String) {
    if stdout.len() != expected_stdout.len() {
        return (false, format!("行数が一致しません: 実行結果の行数: {}, 期待される出力の行数: {}", stdout.len(), expected_stdout.len()));
    }
    for (i, line) in stdout.iter().enumerate() {
        if line.trim() != expected_stdout[i].trim() {
            return (false, format!("行 {} が一致しません: 実行結果: '{}', 期待される出力: '{}'", i + 1, line, expected_stdout[i]));
        }
    }
    return (true, "".to_string())
}