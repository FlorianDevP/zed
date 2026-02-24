use gh_workflow::*;

use crate::tasks::workflows::{
    runners::{self},
    steps::{self, NamedJob, dependant_job, named},
    vars::{self, JobOutput, StepOutput},
};

pub fn sync_with_remote() -> Workflow {
    let (check, has_changes) = check_main();
    let sync_main = sync_main(&[&check], &has_changes);
    let sync_dev = sync_branch(&[&check, &sync_main], "dev", &has_changes);
    let sync_clock = sync_branch(&[&check, &sync_main], "clock", &has_changes);

    named::workflow()
        .run_name("sync fork with remote")
        .on(Event::default()
            .workflow_dispatch(WorkflowDispatch::default())
            .add_cron_schedule("0 0 * * *")) // will run daily at midnight
        .concurrency(
            Concurrency::new(Expression::new(format!("${{{{ github.workflow }}}}")))
                .cancel_in_progress(true),
        )
        .add_job(check.name.clone(), check.job)
        .add_job(sync_main.name.clone(), sync_main.job)
        .add_job(sync_dev.name.clone(), sync_dev.job)
        .add_job(sync_clock.name.clone(), sync_clock.job)
}

pub fn diff_against_base(base_branch: impl AsRef<str>) -> Step<Run> {
    let base_branch = base_branch.as_ref();
    custom_named::bash(indoc::formatdoc! {r#"
        git diff --shortstat "{base_branch}"
    "#})
}

pub fn check_against_base(base_branch: impl AsRef<str>) -> (Step<Run>, StepOutput) {
    let base_branch = base_branch.as_ref();
    let step = custom_named::bash(indoc::formatdoc! {r#"
        if git diff --quiet "{base_branch}"; then
            echo "No upstream changes"
            echo "has_changes=false" >> "$GITHUB_OUTPUT"
        else
            git diff --shortstat "{base_branch}"
            echo "has_changes=true" >> "$GITHUB_OUTPUT"
        fi
    "#})
    .id("diff-against-base");

    let output = StepOutput::new(&step, "has_changes");
    (step, output)
}

pub fn configure_git() -> Step<Run> {
    named::bash(indoc::indoc! {r#"
        git config user.name "github-actions[bot]"
        git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    "#})
}

pub fn setup_upstream(upstream: impl AsRef<str>, base_branch: impl AsRef<str>) -> Step<Run> {
    custom_named::bash(indoc::formatdoc! {r#"
        git remote add upstream "https://github.com/{}.git"
        git fetch --depth=350 upstream "{}"
    "#, upstream.as_ref(), base_branch.as_ref()})
    .id("setup-upstream")
}

pub fn checkout_origin_branch(branch: impl AsRef<str>) -> Step<Run> {
    let branch = branch.as_ref();
    custom_named::bash(format!("git checkout -B {branch} origin/{branch}"))
}

/// Hard Reset a branch with upstream
pub fn hard_reset(base: impl AsRef<str>, branch: impl AsRef<str>) -> Step<Run> {
    let base = base.as_ref();
    let branch = branch.as_ref();
    custom_named::bash(indoc::formatdoc! {r#"
        git checkout -B {branch} origin/{branch}
        git reset --hard {base}
    "#})
}

pub fn rebase(base: impl AsRef<str>, branch: impl AsRef<str>) -> Step<Run> {
    let base = base.as_ref();
    let branch = branch.as_ref();
    custom_named::bash(indoc::formatdoc! {r#"
        git checkout -B {branch} origin/{branch}
        git pull --rebase origin {base}
    "#})
}

pub fn push(branch: impl AsRef<str>) -> Step<Run> {
    custom_named::bash(format!("git push origin {}", branch.as_ref()))
}

pub fn force_push_with_lease(branch: impl AsRef<str>) -> Step<Run> {
    custom_named::bash(format!(
        "git push --force-with-lease origin {}",
        branch.as_ref(),
    ))
}

pub fn force_push_if_includes(branch: impl AsRef<str>) -> Step<Run> {
    custom_named::bash(format!(
        "git push --force-if-includes origin {}",
        branch.as_ref(),
    ))
}

const GH_TOKEN: (&'static str, &'static str) = ("GH_TOKEN", "${{ github.token }}");

fn inverted_repository_owner_guard_expression() -> Expression {
    Expression::new(format!(
        "!{}",
        crate::tasks::workflows::steps::DEFAULT_REPOSITORY_OWNER_GUARD
    ))
}

pub fn check_main() -> (NamedJob, JobOutput) {
    let (diff, has_changes) = check_against_base("upstream/main");
    let job = named::job(
        Job::default()
            .runs_on(runners::GITHUB_LINUX_SLIM)
            .permissions(Permissions::default().contents(Level::Read))
            .outputs([(has_changes.name.to_owned(), has_changes.to_string())])
            .add_env(GH_TOKEN)
            .cond(inverted_repository_owner_guard_expression())
            .add_step(
                steps::CheckoutStep::default()
                    .with_deep_history_on_non_main()
                    .with_ref("main"),
            )
            .add_step(configure_git())
            .add_step(setup_upstream("zed-industries/zed", "main"))
            .add_step(checkout_origin_branch("main"))
            .add_step(diff),
    );

    let has_changes = has_changes.as_job_output(&job);
    (job, has_changes)
}

pub fn sync_main(deps: &[&NamedJob], has_changes: &JobOutput) -> NamedJob {
    named::job(
        dependant_job(deps)
            .runs_on(runners::GITHUB_LINUX_SLIM)
            .cond(Expression::new(format!("{} == 'true'", has_changes.expr())))
            .add_step(
                steps::CheckoutStep::default()
                    .with_deep_history_on_non_main()
                    .with_ref("main")
                    .with_ssh_key(vars::FORK_DEPLOY_KEY),
            )
            .add_step(configure_git())
            .add_step(setup_upstream("zed-industries/zed", "main"))
            .add_step(hard_reset("upstream/main", "main"))
            .add_step(force_push_if_includes("main")),
    )
}

pub fn sync_branch(
    deps: &[&NamedJob],
    branch: impl AsRef<str>,
    has_changes: &JobOutput,
) -> NamedJob {
    let branch = branch.as_ref();

    custom_named::job_with_param(
        branch,
        dependant_job(deps)
            .runs_on(runners::GITHUB_LINUX_SLIM)
            .cond(Expression::new(format!("{} == 'true'", has_changes.expr())))
            .name(format!("sync {branch}"))
            .add_step(
                steps::CheckoutStep::default()
                    .with_deep_history_on_non_main()
                    .with_ref(branch)
                    .with_ssh_key(vars::FORK_DEPLOY_KEY),
            )
            .add_step(configure_git())
            .add_step(rebase("main", branch))
            .add_step(diff_against_base(format!("origin/{branch}")))
            .add_step(force_push_with_lease(branch)),
    )
}

mod custom_named {
    use gh_workflow::{Job, JobType, Run, Step};

    use crate::tasks::workflows::steps::{NamedJob, named};

    /// Returns the function name N callers above in the stack
    /// (typically 1).
    /// This only works because xtask always runs debug builds.
    pub fn function_name(i: usize) -> String {
        let name = named::function_name(i + 1);
        name.split_once("<")
            .and_then(|(name, _)| Some(name.to_owned()))
            .unwrap_or(name)
    }

    /// Returns a bash-script step with the same name as the enclosing function.
    /// (You shouldn't inline this function into the workflow definition, you must
    /// wrap it in a new function.)
    pub fn bash(script: impl AsRef<str>) -> Step<Run> {
        Step::new(function_name(1)).run(script.as_ref())
    }

    /// Returns a Job with the same name as the enclosing function.
    /// (note job names may not contain `::`)
    pub fn job<J: JobType>(job: Job<J>) -> NamedJob<J> {
        NamedJob {
            name: function_name(1).split("::").last().unwrap().to_owned(),
            job,
        }
    }

    /// Returns a Job with the same name as the enclosing function.
    /// (note job names may not contain `::`)
    pub fn job_with_param<J: JobType>(param: impl AsRef<str>, job: Job<J>) -> NamedJob<J> {
        let name = function_name(1);
        let name = name.split("::").last().unwrap();
        NamedJob {
            name: format!("{}_{}", name, param.as_ref()),
            job,
        }
    }
}
