--  // Copyright 2022 OnSight Tech Services LLC
--  // SPDX-License-Identifier: BSD-3-Clause

ALTER TABLE auth_status
    ADD COLUMN revoked BOOLEAN NOT NULL DEFAULT FALSE;
