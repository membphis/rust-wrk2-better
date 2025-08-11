use clap::{Arg, Command};
use regex::Regex;
use std::process::Command as ProcessCommand;

#[derive(Debug)]
struct Wrk2Result {
    total_requests: u64,
    duration: String,
    data_read: String,
    requests_per_sec: f64,
    transfer_per_sec: String,
    uncorrected_latency: Vec<(String, String)>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数（适配clap 4.0+版本）
    let matches = Command::new("wrk2-wrapper")
        .version("1.0")
        .author("Your Name")
        .about("Wraps wrk2 command and extracts key metrics")
        .arg(Arg::new("wrk2_args")
            .help("Arguments to pass to wrk2 (excluding -R and -U which are handled automatically)")
            .action(clap::ArgAction::Append)  // 替代 multiple_occurrences(true)
            .allow_hyphen_values(true))       // 允许接收以连字符开头的参数
        .get_matches();

    // 收集 wrk2 参数（适配clap 4.0+版本）
    let mut wrk2_args: Vec<&str> = matches
        .get_many::<String>("wrk2_args")
        .map(|v| v.map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // 检查是否包含 -R 参数
    let has_r_flag = wrk2_args.iter().any(|&arg| arg == "-R");
    if !has_r_flag {
        wrk2_args.push("-R");
        wrk2_args.push("99999999");
    }

    // 检查是否包含 -U 参数
    let has_u_flag = wrk2_args.iter().any(|&arg| arg == "-U");
    if !has_u_flag {
        wrk2_args.push("-U");
    }

    // 检查是否包含 -v 参数
    let has_v_flag = wrk2_args.iter().any(|&arg| arg == "-v");

    println!("wrk2 {}", wrk2_args.join(" "));

    // 执行 wrk2 命令
    let output = ProcessCommand::new("wrk2")
        .args(&wrk2_args)
        .output()
        .expect("Failed to execute wrk2 command");
    if !output.status.success() {
        eprintln!("wrk2 command failed with exit code: {:?}", output.status.code());
        eprintln!("Error output: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    // 将输出转换为字符串
    let output_str = String::from_utf8_lossy(&output.stdout);
    if has_v_flag {
        println!("======\n{}", output_str);
    }

    // 解析结果
    let result = parse_wrk2_output(&output_str)?;
    
    // 打印提取的结果
    println!("\nPerformance Results:");
    println!("---------------------");
    println!("Totals      : {}", format_u64_with_commas(result.total_requests));
    println!("Duration    : {}", result.duration);
    println!("Data read   : {}", result.data_read);
    println!("Requests/sec: {}", format_u64_with_commas(result.requests_per_sec as u64));
    println!("Transfer/sec: {}", result.transfer_per_sec);
    println!("\nUncorrected Latency:");
    println!("---------------------");
    for (percentile, latency) in result.uncorrected_latency {
        println!("{:>8}%: {}", percentile, latency);
    }

    Ok(())
}

fn parse_wrk2_output(output: &str) -> Result<Wrk2Result, Box<dyn std::error::Error>> {
    // 正则表达式匹配汇总结果
    let summary_re = Regex::new(r"(\d+) requests in (\d+\.\d+\w+), (\d+\.\d+\w+) read")?;
    let req_sec_re = Regex::new(r"Requests/sec:\s+(\d+\.\d+)")?;
    let transfer_sec_re = Regex::new(r"Transfer/sec:\s+(\d+\.\d+\w+)")?;

    // println!("output: {}", output);

    // 提取汇总信息
    let summary_cap = match summary_re.captures(output) {
        Some(cap) => cap,
        None => {
            // 可以在这里添加更详细的错误处理或调试信息
            eprintln!("Debug: Could not find summary pattern in output");
            eprintln!("==========\n{}==========", output);
            return Err("Could not find summary information in output".into());
        }
    };
    
    let total_requests = summary_cap[1].parse()?;
    let duration = summary_cap[2].to_string();
    let data_read = summary_cap[3].to_string();

    // 提取每秒请求数
    let req_sec_cap = req_sec_re.captures(output)
        .ok_or("Could not find Requests/sec in output")?;
    let requests_per_sec = req_sec_cap[1].parse()?;

    // 提取每秒传输数据量
    let transfer_sec_cap = transfer_sec_re.captures(output)
        .ok_or("Could not find Transfer/sec in output")?;
    let transfer_per_sec = transfer_sec_cap[1].to_string();

    // 找到 Uncorrected Latency 部分
    let uncorrected_start = output.find("Uncorrected Latency")
        .ok_or("Could not find Uncorrected Latency section")?;
    let uncorrected_part = &output[uncorrected_start..];

    // 提取延迟数据
    // let uncorrected_latency = extract_uncorrected_latency(uncorrected_part)?;
    let uncorrected_latency = match extract_uncorrected_latency(uncorrected_part) {
        Ok(latency_data) => {
            // 正常情况：检查数据是否合理
            if latency_data.is_empty() {
                return Err("Extracted empty latency data set".into());
            } else {
                // println!("调试: 成功提取 {} 条延迟数据", latency_data.len());
                latency_data
            }
        }
        Err(e) => {
            // 错误情况：记录详细错误信息并返回
            // eprintln!("错误: 提取延迟数据时发生错误 - {}", e);
            // eprintln!("错误上下文: 尝试从以下内容提取数据: {}", uncorrected_part);回

            return Err(format!("Failed to extract uncorrected latency data: {}", e).into());
        }
    };

    Ok(Wrk2Result {
        total_requests,
        duration,
        data_read,
        requests_per_sec,
        transfer_per_sec,
        uncorrected_latency,
    })
}

// 提取Uncorrected Latency部分的所有百分比数据
fn extract_uncorrected_latency(output: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    // println!("uncorrected_part: {}", output);
    // 找到Uncorrected Latency部分的开始
    let start_idx = output.find("Uncorrected Latency")
        .ok_or("Could not find Uncorrected Latency section")?;
    
    // 提取从开始位置到下一个空行或"Detailed Percentile spectrum"的内容
    let latency_section = {
        let section_start = start_idx + "Uncorrected Latency".len();
        let section_end = output[section_start..].find("\n\n")
            .or_else(|| output[section_start..].find("Detailed Percentile spectrum"))
            .ok_or("Could not find end of Uncorrected Latency section")?;
        
        &output[section_start..section_start + section_end]
    };

    // 正则表达式匹配百分比和延迟值
    let latency_re = Regex::new(r"(\d+\.\d+|\d+)%\s+(.+)")?;
    let mut results = Vec::new();

    // 逐行处理
    for line in latency_section.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(cap) = latency_re.captures(line) {
            let percentile = cap[1].to_string();
            let latency = cap[2].to_string();
            results.push((percentile, latency));
        }
    }

    if results.is_empty() {
        return Err("No latency data found in Uncorrected Latency section".into());
    }

    Ok(results)
}

fn format_str_with_commas(s: String) -> String {
    let mut result = String::new();
    let mut count = 0;

    // 从后往前遍历，每三位添加一个逗号
    for c in s.chars().rev() {
        if count != 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    // 反转回来得到正确的顺序
    result.chars().rev().collect()
}


fn format_u64_with_commas(n: u64) -> String {
    let s = n.to_string();
    format_str_with_commas(s)
}
