/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![deny(warnings)]

mod mem_writes_bonsai_hg_mapping;
mod mem_writes_changesets;

use anyhow::Error;
use blobrepo_override::DangerousOverride;
use blobstore::Blobstore;
use bonsai_hg_mapping::BonsaiHgMapping;
use cacheblob::{dummy::DummyLease, LeaseOps, MemWritesBlobstore};
use changesets::Changesets;
use clap::{Arg, SubCommand};
use cmdlib::{
    args::{self, RepoRequirement},
    helpers::block_execute,
};
use context::CoreContext;
use fbinit::FacebookInit;
use git2::{Oid, Repository};
use import_tools::{
    FullRepoImport, GitRangeImport, GitimportPreferences, GitimportTarget, ImportMissingForCommit,
};
use linked_hash_map::LinkedHashMap;
use mononoke_types::{BonsaiChangeset, ChangesetId};
use slog::info;
use std::path::Path;
use std::sync::Arc;

use crate::mem_writes_bonsai_hg_mapping::MemWritesBonsaiHgMapping;
use crate::mem_writes_changesets::MemWritesChangesets;

// Refactor this a bit. Use a thread pool for git operations. Pass that wherever we use store repo.
// Transform the walk into a stream of commit + file changes.

const SUBCOMMAND_FULL_REPO: &str = "full-repo";
const SUBCOMMAND_GIT_RANGE: &str = "git-range";
const SUBCOMMAND_MISSING_FOR_COMMIT: &str = "missing-for-commit";

const ARG_GIT_REPOSITORY_PATH: &str = "git-repository-path";
const ARG_DERIVE_TREES: &str = "derive-trees";
const ARG_DERIVE_HG: &str = "derive-hg";
const ARG_HGGIT_COMPATIBILITY: &str = "hggit-compatibility";
const ARG_BONSAI_GIT_MAPPING: &str = "bonsai-git-mapping";
const ARG_SUPPRESS_REF_MAPPING: &str = "suppress-ref-mapping";

const ARG_GIT_FROM: &str = "git-from";
const ARG_GIT_TO: &str = "git-to";

const ARG_GIT_COMMIT: &str = "git-commit";

#[fbinit::main]
fn main(fb: FacebookInit) -> Result<(), Error> {
    let app = args::MononokeAppBuilder::new("Mononoke Git Importer")
        .with_repo_required(RepoRequirement::ExactlyOne)
        .build()
        .arg(
            Arg::with_name(ARG_DERIVE_TREES)
                .long(ARG_DERIVE_TREES)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(ARG_DERIVE_HG)
                .long(ARG_DERIVE_HG)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(ARG_HGGIT_COMPATIBILITY)
                .long(ARG_HGGIT_COMPATIBILITY)
                .help("Set commit extras for hggit compatibility")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(ARG_BONSAI_GIT_MAPPING)
                .long(ARG_BONSAI_GIT_MAPPING)
                .help("For each created commit also create a bonsai<->git commit mapping.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(ARG_SUPPRESS_REF_MAPPING)
                .long(ARG_SUPPRESS_REF_MAPPING)
                .help("This is used to suppress the printing of the potentially really long git Reference -> BonzaiID mapping.")
                .required(false)
                .takes_value(false),
        )
        .arg(Arg::with_name(ARG_GIT_REPOSITORY_PATH).help("Path to a git repository to import"))
        .subcommand(SubCommand::with_name(SUBCOMMAND_FULL_REPO))
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_GIT_RANGE)
                .arg(
                    Arg::with_name(ARG_GIT_FROM)
                        .required(true)
                        .takes_value(true),
                )
                .arg(Arg::with_name(ARG_GIT_TO).required(true).takes_value(true)),
        )
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_MISSING_FOR_COMMIT).arg(
                Arg::with_name(ARG_GIT_COMMIT)
                    .required(true)
                    .takes_value(true),
            ),
        );

    let mut prefs = GitimportPreferences::default();

    let matches = app.get_matches();

    // if we are readonly, then we'll set up some overrides to still be able to do meaningful
    // things below.
    let dry_run = args::parse_readonly_storage(&matches).0;

    if matches.is_present(ARG_DERIVE_TREES) {
        prefs.enable_derive_trees();
    }

    if matches.is_present(ARG_DERIVE_HG) {
        prefs.enable_derive_hg();
    }

    if matches.is_present(ARG_HGGIT_COMPATIBILITY) {
        prefs.enable_hggit_compatibility();
    }

    if matches.is_present(ARG_BONSAI_GIT_MAPPING) {
        prefs.enable_bonsai_git_mapping();
    }

    let path = Path::new(matches.value_of(ARG_GIT_REPOSITORY_PATH).unwrap());

    args::init_cachelib(fb, &matches);
    let logger = args::init_logging(fb, &matches)?;
    args::init_config_store(fb, &logger, &matches)?;
    let ctx = CoreContext::new_with_logger(fb, logger.clone());

    let repo = args::create_repo(fb, &logger, &matches);
    block_execute(
        async {
            let repo = repo.await?;

            let repo = if dry_run {
                repo.dangerous_override(|blobstore| -> Arc<dyn Blobstore> {
                    Arc::new(MemWritesBlobstore::new(blobstore))
                })
                .dangerous_override(|changesets| -> Arc<dyn Changesets> {
                    Arc::new(MemWritesChangesets::new(changesets))
                })
                .dangerous_override(|bonsai_hg_mapping| -> Arc<dyn BonsaiHgMapping> {
                    Arc::new(MemWritesBonsaiHgMapping::new(bonsai_hg_mapping))
                })
                .dangerous_override(|_| Arc::new(DummyLease {}) as Arc<dyn LeaseOps>)
            } else {
                repo
            };

            let git_repo = Repository::open(&path)?;

            let target: Box<dyn GitimportTarget> = match matches.subcommand() {
                (SUBCOMMAND_FULL_REPO, Some(..)) => Box::new(FullRepoImport {}),
                (SUBCOMMAND_GIT_RANGE, Some(range_matches)) => {
                    let from = range_matches.value_of(ARG_GIT_FROM).unwrap().parse()?;
                    let to = range_matches.value_of(ARG_GIT_TO).unwrap().parse()?;
                    Box::new(GitRangeImport::new(from, to, &ctx, &repo).await?)
                }
                (SUBCOMMAND_MISSING_FOR_COMMIT, Some(matches)) => {
                    let commit = matches.value_of(ARG_GIT_COMMIT).unwrap().parse()?;
                    Box::new(ImportMissingForCommit::new(commit, &ctx, &repo, &git_repo).await?)
                }
                _ => {
                    return Err(Error::msg("A valid subcommand is required"));
                }
            };

            let gitimport_result: LinkedHashMap<Oid, (ChangesetId, BonsaiChangeset)> =
                import_tools::gitimport(&ctx, &repo, &path, &*target, prefs).await?;

            if !matches.is_present(ARG_SUPPRESS_REF_MAPPING) {
                for reference in git_repo.references()? {
                    let reference = reference?;
                    let commit = reference.peel_to_commit()?;
                    let bcs_id = gitimport_result.get(&commit.id()).map(|e| e.0);
                    info!(ctx.logger(), "Ref: {:?}: {:?}", reference.name(), bcs_id);
                }
            }

            Ok(())
        },
        fb,
        "gitimport",
        &logger,
        &matches,
        cmdlib::monitoring::AliveService,
    )
}
