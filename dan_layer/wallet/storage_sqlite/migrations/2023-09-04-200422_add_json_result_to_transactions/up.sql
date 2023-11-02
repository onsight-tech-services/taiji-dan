--  // Copyright 2022 OnSight Tech Services LLC
--  // SPDX-License-Identifier: BSD-3-Clause

ALTER TABLE transactions
    ADD COLUMN json_result TEXT NULL;
