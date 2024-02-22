/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! ------------
//! IMPORTANT!!!
//! ------------
//! Do not change the order of the fields! Changing the order of the fields
//! results in compatible but *not* identical serializations, so hashes will
//! change.
//! ------------
//! IMPORTANT!!!
//! ------------

namespace py3 eden.mononoke.mononoke_types

include "eden/mononoke/mononoke_types/serialization/data.thrift"
include "eden/mononoke/mononoke_types/serialization/id.thrift"
include "eden/mononoke/mononoke_types/serialization/path.thrift"
include "eden/mononoke/mononoke_types/serialization/time.thrift"
include "eden/mononoke/mononoke_types/serialization/bonsai.thrift"
include "eden/mononoke/mononoke_types/serialization/sharded_map.thrift"

union RawBundle2 {
  1: binary Bytes;
}

struct RedactionKeyList {
  // List of keys to be redacted
  1: list<string> keys;
} (rust.exhaustive)
