use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use chrono::Local;
// use serde::Serialize;

use csv::WriterBuilder;
use walkdir::WalkDir;  // ディレクトリとファイルを再起的に操作するためのクレート
use anyhow::{Result, Context, anyhow};  // エラー処理をいい感じにするクレート


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
    
    // 標準入力からフォルダパスをうけとる
    println!("Pythonファイルが含まれるフォルダパスを入力してください:");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let folder_path = input.trim();
    // 現在のディレクトリ直下にそのフォルダがあるか確認
    let folder_path = Path::new(folder_path);
    if !folder_path.exists() || !folder_path.is_dir() {
        return Err(anyhow!("指定されたパスは有効なディレクトリではありません: {:?}", folder_path));
    }
    
    // 入力ファイルと出力ファイルを読み込む
    let input_content = fs::read_to_string("input.txt")
        .context("入力ファイル(input.txt)の読み込みに失敗しました")?;
    let expected_output = fs::read_to_string("output.txt")
        .context("出力ファイル(output.txt)の読み込みに失敗しました")?;
    
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
        
        
        match test_python_file(python_file, &input_content, &expected_output) {
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



fn test_python_file(file_path: &Path, input: &str, expected_output: &str) -> Result<(String, String)> {
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
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    
    // 実行結果と期待される出力を比較
    if stdout == expected_output.trim() {
        Ok(("PASS".to_string(), "".to_string()))
    } else {
        // 失敗時はエラー詳細を含む
        let error_message = if !stderr.is_empty() {
            stderr
        } else {
            format!("期待された出力: '{}', 実際の出力: '{}'", expected_output.trim(), stdout)
        };
        
        Ok(("FAIL".to_string(), error_message))
    }
}