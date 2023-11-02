--  // Copyright 2022 OnSight Tech Services LLC
--  // SPDX-License-Identifier: BSD-3-Clause

alter table accounts
add column is_default boolean not null default 0;
