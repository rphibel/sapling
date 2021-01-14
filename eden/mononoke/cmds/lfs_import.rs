/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{Error, Result};
use borrowed::borrowed;
use bytes::Bytes;
use clap::Arg;
use cmdlib::{args, helpers::block_execute};
use context::CoreContext;
use fbinit::FacebookInit;
use futures::stream::{self, TryStreamExt};
use lfs_import_lib::lfs_upload;
use mercurial_types::blobs::File;

const NAME: &str = "lfs_import";

const ARG_LFS_HELPER: &str = "lfs-helper";
const ARG_CONCURRENCY: &str = "concurrency";
const ARG_POINTERS: &str = "pointers";
const ARG_NO_CREATE: &str = "no-create";

const DEFAULT_CONCURRENCY: usize = 16;

#[fbinit::main]
fn main(fb: FacebookInit) -> Result<()> {
    let app = args::MononokeAppBuilder::new(NAME)
        .with_advanced_args_hidden()
        .build()
        .about("Import LFS blobs")
        .arg(
            Arg::with_name(ARG_CONCURRENCY)
                .long("concurrency")
                .takes_value(true)
                .help("The number of OIDs to process in parallel"),
        )
        .arg(
            Arg::with_name(ARG_NO_CREATE)
                .long(ARG_NO_CREATE)
                .takes_value(false)
                .required(false)
                .help("If provided won't create a new repo"),
        )
        .arg(
            Arg::with_name(ARG_LFS_HELPER)
                .required(true)
                .takes_value(true)
                .help("LFS Helper"),
        )
        .arg(
            Arg::with_name(ARG_POINTERS)
                .takes_value(true)
                .required(true)
                .min_values(1)
                .help("Raw LFS pointers to be imported"),
        );

    let matches = app.get_matches();
    args::init_cachelib(fb, &matches);

    let logger = args::init_logging(fb, &matches)?;
    args::init_config_store(fb, &logger, &matches)?;
    let ctx = CoreContext::new_with_logger(fb, logger.clone());
    let lfs_helper = matches.value_of(ARG_LFS_HELPER).unwrap().to_string();

    let concurrency: usize = matches
        .value_of(ARG_CONCURRENCY)
        .map_or(Ok(DEFAULT_CONCURRENCY), |j| j.parse())
        .map_err(Error::from)?;

    let entries: Vec<_> = matches
        .values_of(ARG_POINTERS)
        .unwrap()
        .into_iter()
        .map(|e| File::new(Bytes::copy_from_slice(e.as_bytes()), None, None).get_lfs_content())
        .collect();

    let import = {
        let matches = &matches;
        let logger = &logger;
        async move {
            let blobrepo = if matches.is_present(ARG_NO_CREATE) {
                args::open_repo(fb, logger, &matches).await?
            } else {
                args::create_repo(fb, logger, &matches).await?
            };
            stream::iter(entries)
                .try_for_each_concurrent(concurrency, {
                    borrowed!(ctx, blobrepo, lfs_helper);
                    move |lfs| async move {
                        lfs_upload(ctx, blobrepo, lfs_helper, &lfs).await?;
                        Ok(())
                    }
                })
                .await
        }
    };

    block_execute(
        import,
        fb,
        NAME,
        &logger,
        &matches,
        cmdlib::monitoring::AliveService,
    )
}
