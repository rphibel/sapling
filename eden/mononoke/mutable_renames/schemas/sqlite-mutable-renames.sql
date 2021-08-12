/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

CREATE TABLE mutable_renames(
   `id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
   `repo_id` INT UNSIGNED NOT NULL,
   `dst_cs_id` VARBINARY(32) NOT NULL,
   `dst_path` VARBINARY(4096) NOT NULL,
   `src_cs_id` VARBINARY(32) NOT NULL,
   `src_path` VARBINARY(4096) NOT NULL,
   `src_unode_id` VARBINARY(32) NOT NULL,
   `is_tree` BIT NOT NULL,
   UNIQUE (`repo_id`, `dst_cs_id`, `dst_path`)
);
