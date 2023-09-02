// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use anyhow::Result;
use futures::AsyncReadExt;
use sha2::Digest;
use sha2::Sha256;

use crate::*;

pub fn behavior_read_only_tests(op: &Operator) -> Vec<Trial> {
    let cap = op.info().full_capability();

    if !cap.read || cap.write {
        return vec![];
    }

    async_trials!(
        op,
        test_read_only_stat_file_and_dir,
        test_read_only_stat_special_chars,
        test_read_only_stat_not_cleaned_path,
        test_read_only_stat_not_exist,
        test_read_only_stat_with_if_match,
        test_read_only_stat_with_if_none_match,
        test_read_only_stat_root,
        test_read_only_read_full,
        test_read_only_read_full_with_special_chars,
        test_read_only_read_with_range,
        test_read_only_reader_with_range,
        test_read_only_reader_from,
        test_read_only_reader_tail,
        test_read_only_read_not_exist,
        test_read_only_read_with_dir_path,
        test_read_only_read_with_if_match,
        test_read_only_read_with_if_none_match
    )
}

/// Stat normal file and dir should return metadata
pub async fn test_read_only_stat_file_and_dir(op: Operator) -> Result<()> {
    let meta = op.stat("normal_file.txt").await?;
    assert_eq!(meta.mode(), EntryMode::FILE);
    assert_eq!(meta.content_length(), 262144);

    let meta = op.stat("normal_dir/").await?;
    assert_eq!(meta.mode(), EntryMode::DIR);

    Ok(())
}

/// Stat special file and dir should return metadata
pub async fn test_read_only_stat_special_chars(op: Operator) -> Result<()> {
    let meta = op.stat("special_file  !@#$%^&()_+-=;',.txt").await?;
    assert_eq!(meta.mode(), EntryMode::FILE);
    assert_eq!(meta.content_length(), 262144);

    let meta = op.stat("special_dir  !@#$%^&()_+-=;',/").await?;
    assert_eq!(meta.mode(), EntryMode::DIR);

    Ok(())
}

/// Stat not cleaned path should also succeed.
pub async fn test_read_only_stat_not_cleaned_path(op: Operator) -> Result<()> {
    let meta = op.stat("//normal_file.txt").await?;
    assert_eq!(meta.mode(), EntryMode::FILE);
    assert_eq!(meta.content_length(), 262144);

    Ok(())
}

/// Stat not exist file should return NotFound
pub async fn test_read_only_stat_not_exist(op: Operator) -> Result<()> {
    let path = uuid::Uuid::new_v4().to_string();

    let meta = op.stat(&path).await;
    assert!(meta.is_err());
    assert_eq!(meta.unwrap_err().kind(), ErrorKind::NotFound);

    Ok(())
}

/// Stat with if_match should succeed, else get a ConditionNotMatch error.
pub async fn test_read_only_stat_with_if_match(op: Operator) -> Result<()> {
    if !op.info().full_capability().stat_with_if_match {
        return Ok(());
    }

    let path = "normal_file.txt";

    let meta = op.stat(path).await?;
    assert_eq!(meta.mode(), EntryMode::FILE);
    assert_eq!(meta.content_length(), 262144);

    let res = op.stat_with(path).if_match("invalid_etag").await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), ErrorKind::ConditionNotMatch);

    let result = op
        .stat_with(path)
        .if_match(meta.etag().expect("etag must exist"))
        .await;
    assert!(result.is_ok());

    Ok(())
}

/// Stat with if_none_match should succeed, else get a ConditionNotMatch.
pub async fn test_read_only_stat_with_if_none_match(op: Operator) -> Result<()> {
    if !op.info().full_capability().stat_with_if_none_match {
        return Ok(());
    }

    let path = "normal_file.txt";

    let meta = op.stat(path).await?;
    assert_eq!(meta.mode(), EntryMode::FILE);
    assert_eq!(meta.content_length(), 262144);

    let res = op
        .stat_with(path)
        .if_none_match(meta.etag().expect("etag must exist"))
        .await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), ErrorKind::ConditionNotMatch);

    let res = op.stat_with(path).if_none_match("invalid_etag").await?;
    assert_eq!(res.mode(), meta.mode());
    assert_eq!(res.content_length(), meta.content_length());

    Ok(())
}

/// Root should be able to stat and returns DIR.
pub async fn test_read_only_stat_root(op: Operator) -> Result<()> {
    let meta = op.stat("").await?;
    assert_eq!(meta.mode(), EntryMode::DIR);

    let meta = op.stat("/").await?;
    assert_eq!(meta.mode(), EntryMode::DIR);

    Ok(())
}

/// Read full content should match.
pub async fn test_read_only_read_full(op: Operator) -> Result<()> {
    let bs = op.read("normal_file.txt").await?;
    assert_eq!(bs.len(), 262144, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "e7541d0f50d2d5c79dc41f28ccba8e0cdfbbc8c4b1aa1a0110184ef0ef67689f",
        "read content"
    );

    Ok(())
}

/// Read full content should match.
pub async fn test_read_only_read_full_with_special_chars(op: Operator) -> Result<()> {
    let bs = op.read("special_file  !@#$%^&()_+-=;',.txt").await?;
    assert_eq!(bs.len(), 262144, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "e7541d0f50d2d5c79dc41f28ccba8e0cdfbbc8c4b1aa1a0110184ef0ef67689f",
        "read content"
    );

    Ok(())
}

/// Read full content should match.
pub async fn test_read_only_read_with_range(op: Operator) -> Result<()> {
    let bs = op.read_with("normal_file.txt").range(1024..2048).await?;
    assert_eq!(bs.len(), 1024, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "28786fb63abfe5545479e4f50da853652d1d67b88be5553c265ede4022774913",
        "read content"
    );

    Ok(())
}

/// Read range should match.
pub async fn test_read_only_reader_with_range(op: Operator) -> Result<()> {
    let mut r = op.reader_with("normal_file.txt").range(1024..2048).await?;

    let mut bs = Vec::new();
    r.read_to_end(&mut bs).await?;

    assert_eq!(bs.len(), 1024, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "28786fb63abfe5545479e4f50da853652d1d67b88be5553c265ede4022774913",
        "read content"
    );

    Ok(())
}

/// Read from should match.
pub async fn test_read_only_reader_from(op: Operator) -> Result<()> {
    let mut r = op.reader_with("normal_file.txt").range(261120..).await?;

    let mut bs = Vec::new();
    r.read_to_end(&mut bs).await?;

    assert_eq!(bs.len(), 1024, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "81fa400e85baa2a5c7006d77d4320b73d36222974b923e03ed9891580f989e2a",
        "read content"
    );

    Ok(())
}

/// Read tail should match.
pub async fn test_read_only_reader_tail(op: Operator) -> Result<()> {
    let mut r = op.reader_with("normal_file.txt").range(..1024).await?;

    let mut bs = Vec::new();
    r.read_to_end(&mut bs).await?;

    assert_eq!(bs.len(), 1024, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "81fa400e85baa2a5c7006d77d4320b73d36222974b923e03ed9891580f989e2a",
        "read content"
    );

    Ok(())
}

/// Read not exist file should return NotFound
pub async fn test_read_only_read_not_exist(op: Operator) -> Result<()> {
    let path = uuid::Uuid::new_v4().to_string();

    let bs = op.read(&path).await;
    assert!(bs.is_err());
    assert_eq!(bs.unwrap_err().kind(), ErrorKind::NotFound);

    Ok(())
}

/// Read with dir path should return an error.
pub async fn test_read_only_read_with_dir_path(op: Operator) -> Result<()> {
    let path = format!("{}/", uuid::Uuid::new_v4());

    let result = op.read(&path).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::IsADirectory);

    Ok(())
}

/// Read with if_match should match, else get a ConditionNotMatch error.
pub async fn test_read_only_read_with_if_match(op: Operator) -> Result<()> {
    if !op.info().full_capability().read_with_if_match {
        return Ok(());
    }

    let path = "normal_file.txt";

    let meta = op.stat(path).await?;

    let res = op.read_with(path).if_match("invalid_etag").await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), ErrorKind::ConditionNotMatch);

    let bs = op
        .read_with(path)
        .if_match(meta.etag().expect("etag must exist"))
        .await
        .expect("read must succeed");
    assert_eq!(bs.len(), 262144, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "e7541d0f50d2d5c79dc41f28ccba8e0cdfbbc8c4b1aa1a0110184ef0ef67689f",
        "read content"
    );

    Ok(())
}

/// Read with if_none_match should match, else get a ConditionNotMatch error.
pub async fn test_read_only_read_with_if_none_match(op: Operator) -> Result<()> {
    if !op.info().full_capability().read_with_if_none_match {
        return Ok(());
    }

    let path = "normal_file.txt";

    let meta = op.stat(path).await?;

    let res = op
        .read_with(path)
        .if_none_match(meta.etag().expect("etag must exist"))
        .await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), ErrorKind::ConditionNotMatch);

    let bs = op
        .read_with(path)
        .if_none_match("invalid_etag")
        .await
        .expect("read must succeed");
    assert_eq!(bs.len(), 262144, "read size");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bs)),
        "e7541d0f50d2d5c79dc41f28ccba8e0cdfbbc8c4b1aa1a0110184ef0ef67689f",
        "read content"
    );

    Ok(())
}
