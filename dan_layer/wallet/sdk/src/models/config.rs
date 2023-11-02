//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

pub struct Config<T> {
    pub key: String,
    pub value: T,
    pub is_encrypted: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
