-- Copyright 2022 OnSight Tech Services LLC
-- SPDX-License-Identifier: BSD-3-Clause

ALTER TABLE outputs
    ADD COLUMN encrypted_data blob NOT NULL DEFAULT '';
