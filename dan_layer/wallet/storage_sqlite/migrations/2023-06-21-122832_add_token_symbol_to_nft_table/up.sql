--  // Copyright 2022 OnSight Tech Services LLC
--  // SPDX-License-Identifier: BSD-3-Clause

ALTER TABLE non_fungible_tokens
    ADD COLUMN token_symbol TEXT NOT NULL;
