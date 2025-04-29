use std::fs;
use std::collections::HashMap;
use std::path::Path;

use walkdir::WalkDir;
use anyhow::{Result, Context, anyhow};

pub fn get_expected_input(mut input_content: HashMap<String, String>, input_file: &Path) -> Result<()> {
    if input_file.exists() && input_file.is_file() {
        if input_file.extension().map_or(false, |ext| ext == "txt") {
            let input_context = fs::read_to_string(input_file)
                .context(format!("インプットファイルの読み込みに失敗しました: {:?}", input_file))?;
            input_content.insert(String::from("0"), input_context);
        } else {
            return Err(anyhow!("入力するファイルはテキストファイルまたはテキストファイルの入ったディレクトリのみです"));
        }
    } else if input_file.exists() && input_file.is_dir() {
        let mut i = 0; 
        for entry in WalkDir::new(input_file) 
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir() && e.path().extension().map_or(false, |ext| ext == "txt")) 
        {
            let input_context = fs::read_to_string(entry.path())
                .context(format!("インプットファイルの読み込みに失敗しました: {:?}", entry.path()))?;
            input_content.insert(i.to_string(), input_context);

            i += 1;
        }
    }

    Ok(())
}


pub fn get_expected_output(mut output_content: HashMap<String, Vec<String>>, output_file: &Path) -> Result<()> {
   if output_file.exists() && output_file.is_file() {
       if output_file.extension().map_or(false, |ext| ext == "txt") {
           let mut output_vec: Vec<String> = Vec::new();
           let output_context = fs::read_to_string(output_file)
               .context(format!("アウトプットファイルの読み込みに失敗しました: {:?}", output_file))?;

           for line in output_context.lines() {
               output_vec.push(line.to_string());
           }
           output_content.insert(String::from("0"), output_vec);
       } else {
           return Err(anyhow!("出力するファイルはテキストファイルまたはテキストファイルの入ったディレクトリのみです"));
       }
   } else if output_file.exists() && output_file.is_dir() {
       let mut i = 0; 
       for entry in WalkDir::new(output_file) 
           .into_iter()
           .filter_map(Result::ok)
           .filter(|e| !e.file_type().is_dir() && e.path().extension().map_or(false, |ext| ext == "txt")) 
       {
           let mut output_vec: Vec<String> = Vec::new();
           let output_context = fs::read_to_string(entry.path())
               .context(format!("アウトプットファイルの読み込みに失敗しました: {:?}", entry.path()))?;

           for line in output_context.lines() {
               output_vec.push(line.to_string());
           }
           output_content.insert(i.to_string(), output_vec);

           i += 1;
       }
   }

   Ok(())
}