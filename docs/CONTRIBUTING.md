# Contributing to hyperswitch

:tada: First off, thanks for taking the time to contribute!
We are so happy to have you! :tada:

There are opportunities to contribute to hyperswitch at any level.
It doesn't matter if you are just getting started with Rust or are the most
weathered expert, we can use your help.

**No contribution is too small and all contributions are valued.**

This guide will help you get started.
**Do not let this guide intimidate you.**
It should be considered a map to help you navigate the process.

You can also get help with contributing on our [Discord server][discord],
[Slack workspace][slack], or [Discussions][discussions] space.
Please join us!

[discord]: https://discord.gg/wJZ7DVW8mm
[slack]: https://join.slack.com/t/hyperswitch-io/shared_invite/zt-2awm23agh-p_G5xNpziv6yAiedTkkqLg
[discussions]: https://github.com/juspay/hyperswitch/discussions

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Contributing in Issues](#contributing-in-issues)
  - [Asking for General Help](#asking-for-general-help)
  - [Submitting a Bug Report](#submitting-a-bug-report)
  - [Triaging a Bug Report](#triaging-a-bug-report)
  - [Resolving a Bug Report](#resolving-a-bug-report)
- [Pull Requests](#pull-requests)
  - [Cargo Commands](#cargo-commands)
  - [Commits](#commits)
  - [Opening the Pull Request](#opening-the-pull-request)
  - [Discuss and update](#discuss-and-update)
  - [Commit Squashing](#commit-squashing)
- [Reviewing Pull Requests](#reviewing-pull-requests)
  - [Review a bit at a time](#review-a-bit-at-a-time)
  - [Be aware of the person behind the code](#be-aware-of-the-person-behind-the-code)
  - [Abandoned or Stalled Pull Requests](#abandoned-or-stalled-pull-requests)
- [Keeping track of issues and PRs](#keeping-track-of-issues-and-prs)
  - [Area](#area)
  - [Category](#category)
  - [Calls for participation](#calls-for-participation)
  - [Metadata](#metadata)
  - [Priority](#priority)
  - [RFCs](#rfcs)
  - [Status](#status)

## Code of Conduct

The hyperswitch project adheres to the [Rust Code of Conduct][coc].
This describes the _minimum_ behavior expected from all contributors.

[coc]: https://www.rust-lang.org/policies/code-of-conduct

## Contributing in Issues

For any issue, there are fundamentally three ways an individual can contribute:

1. By opening the issue for discussion: For instance, if you believe that you
   have discovered a bug in hyperswitch, creating a new issue in [the
   juspay/hyperswitch issue tracker][issue] is the way to report it.

2. By helping to triage the issue: This can be done by providing supporting
   details (a test case that demonstrates a bug), providing suggestions on how
   to address the issue, or ensuring that the issue is tagged correctly.

3. By helping to resolve the issue: Typically this is done either in the form of
   demonstrating that the issue reported is not a problem after all, or more
   often, by opening a Pull Request that changes some bit of something in
   hyperswitch in a concrete and reviewable manner.

[issue]: https://github.com/juspay/hyperswitch/issues

**Anybody can participate in any stage of contribution**.
We urge you to participate in the discussion around bugs and participate in
reviewing PRs.

### Asking for General Help

If you have reviewed existing documentation and still have questions or are
having problems, you can [open a discussion] asking for help.

In exchange for receiving help, we ask that you contribute back a documentation
PR that helps others avoid the problems that you encountered.

[open a discussion]: https://github.com/juspay/hyperswitch/discussions/new/choose

### Submitting a Bug Report

When opening a new issue in the hyperswitch issue tracker, you will be presented
with a basic template that should be filled in.
If you believe that you have uncovered a bug, please fill out this form,
following the template to the best of your ability.
Do not worry if you cannot answer every detail, just fill in what you can.

The two most important pieces of information we need in order to properly
evaluate the report is a description of the behavior you are seeing and a simple
test case we can use to recreate the problem on our own.
If we cannot recreate the issue, it becomes impossible for us to fix.

See [How to create a Minimal, Complete, and Verifiable example][mcve].

[mcve]: https://stackoverflow.com/help/mcve

### Triaging a Bug Report

Once an issue has been opened, it is not uncommon for there to be discussion
around it.
Some contributors may have differing opinions about the issue, including whether
the behavior being seen is a bug or a feature.
This discussion is part of the process and should be kept focused, helpful, and
professional.

Short, clipped responses — that provide neither additional context nor
supporting detail — are not helpful or professional.
To many, such responses are simply annoying and unfriendly.

Contributors are encouraged to help one another make forward progress as much as
possible, empowering one another to solve issues collaboratively.
If you choose to comment on an issue that you feel either is not a problem that
needs to be fixed, or if you encounter information in an issue that you feel is
incorrect, explain why you feel that way with additional supporting context, and
be willing to be convinced that you may be wrong.
By doing so, we can often reach the correct outcome much faster.

### Resolving a Bug Report

In the majority of cases, issues are resolved by opening a Pull Request.
The process for opening and reviewing a Pull Request is similar to that of
opening and triaging issues, but carries with it a necessary review and approval
workflow that ensures that the proposed changes meet the minimal quality and
functional guidelines of the hyperswitch project.

## Pull Requests

Pull Requests are the way concrete changes are made to the code, documentation,
and dependencies in the hyperswitch repository.

Even tiny pull requests (e.g., one character pull request fixing a typo in API
documentation) are greatly appreciated.
Before making a large change, it is usually a good idea to first open an issue
describing the change to solicit feedback and guidance.
This will increase the likelihood of the PR getting merged.

### Cargo Commands

Due to the extensive use of features in hyperswitch, you will often need to add
extra arguments to many common cargo commands.
This section lists some commonly needed commands.

Some commands just need the `--all-features` argument:

```shell
cargo check --all-features
cargo clippy --all-features
cargo test --all-features
```

The `cargo fmt` command requires the nightly toolchain, as we use a few of the
unstable features:

```shell
cargo +nightly fmt
```

### Commits

It is a recommended best practice to keep your changes as logically grouped as
possible within individual commits.
There is no limit to the number of commits any single Pull Request may have, and
many contributors find it easier to review changes that are split across
multiple commits.

Please adhere to the general guideline that you should never force push to a
publicly shared branch.
Once you have opened your pull request, you should consider your branch publicly shared.
Instead of force pushing you can just add incremental commits;
this is generally easier on your reviewers.
If you need to pick up changes from main, you can merge main into your branch.

A reviewer might ask you to rebase a long-running pull request
in which case force pushing is okay for that request.

Note that squashing at the end of the review process should also not be done,
that can be done when the pull request is integrated via GitHub.


#### Commit message guidelines

Each commit message consists of a header, an optional body, and an optional
footer.

```text
<header>
<BLANK LINE>
<optional body>
<BLANK LINE>
<optional footer>
```

The `header` is mandatory and must conform to the
[commit message header](#commit-message-header) format.

The `body` is optional.
When the body is present it must be at least 20 characters long and must conform
to the [commit message body](#commit-message-body) format.

The `footer` is optional.
The [commit message footer](#commit-message-footer) format describes what the
footer is used for and the structure it must have.

##### Commit message header

```text
<type>(<scope>): <short summary>
│         │             │
│         │             └── Summary in present tense.
|         |                 Not capitalized.
|         |                 No period at the end.
│         │
│         └── Commit Scope: crate name | changelog | config | migrations
|                           | openapi | postman
│
└── Commit Type: build | chore | ci | docs | feat | fix | perf | refactor | test
```

The `<type>` and `<summary>` fields are mandatory, the (`<scope>`) field is
optional.

`<type>` must be one of the following:

- `build`: Changes that affect the build system or external dependencies
  (example scopes: `deps`, `dev-deps`, `metadata`)
- `chore`: Changes such as fixing formatting or addressing warnings or lints, or
  other maintenance changes
- `ci`: Changes to our CI configuration files and scripts (examples: `workflows`,
  `dependabot`, `renovate`)
- `docs`: Documentation only changes
- `feat`: A new feature
- `fix`: A bug fix
- `perf`: A code change that improves performance
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests or correcting existing tests

`<scope>` should be the name of the crate affected (as perceived by the person
reading the changelog generated from commit messages).
The scope can be more specific if the changes are targeted towards the main
crate in the repository (`router`).

The following is the list of supported scopes:

- `masking`
- `router`
- `router_derive`
- `router_env`

There are currently a few exceptions to the "use crate name" rule:

- `changelog`: Used for updating the release notes in the `CHANGELOG.md` file.
  Commonly used with the `docs` commit type
  (e.g. `docs(changelog): generate release notes for v0.4.0 release`).
- `config`: Used for changes which affect the configuration files of any of the
  services.
- `migrations`: Used for changes to the database migration scripts.
- `openapi`: Used for changes to the OpenAPI specification file.
- `postman`: Used for changes to the Postman collection file.
- none/empty string: Useful for test and refactor changes that are done across
  all crates (e.g. `test: add missing unit tests`) and for docs changes that are
  not related to a specific crate (e.g. `docs: fix typo in tutorial`).

Use the `<summary>` field to provide a succinct description of the change:

- Use the imperative, present tense: "change" not "changed" nor "changes".
- Don't capitalize the first letter.
- No period (.) at the end.

##### Commit message body

Just as in the summary, use the imperative, present tense: "fix" not "fixed" nor
"fixes".

Explain the motivation for the change in the commit message body.
This commit message should explain why you are making the change.
You can include a comparison of the previous behavior with the new behavior in
order to illustrate the impact of the change.

##### Commit message footer

The footer can contain information about breaking changes and deprecations and
is also the place to reference GitHub issues, Jira tickets, and other PRs that
this commit closes or is related to.
For example:

```text
BREAKING CHANGE: <breaking change summary>
<BLANK LINE>
<breaking change description + migration instructions>
<BLANK LINE>
<BLANK LINE>
Fixes #<issue number>
```

or

```text
DEPRECATED: <what is deprecated>
<BLANK LINE>
<deprecation description + recommended update path>
<BLANK LINE>
<BLANK LINE>
Closes #<PR number>
```

Breaking Change section should start with the phrase "BREAKING CHANGE: "
followed by a summary of the breaking change, a blank line, and a detailed
description of the breaking change that also includes migration instructions.

Similarly, a Deprecation section should start with "DEPRECATED: " followed by a
short description of what is deprecated, a blank line, and a detailed
description of the deprecation that also mentions the recommended update path.

If the commit reverts a previous commit, it should begin with `revert:`,
followed by the header of the reverted commit.
The content of the commit message body should contain:

- Information about the SHA of the commit being reverted in the following
  format: `This reverts commit <SHA>`.
- A clear description of the reason for reverting the commit message.

Sample commit messages:

1. ```text
   feat(router): add 3ds support to payments core flow


   Implement Redirection flow support. This can be used by any flow that
   requires redirection.

   Fixes #123
   ```

2. ```text
   chore: run formatter
   ```

3. ```text
   fix(config): fix binary name displayed in help message
   ```

_Adapted from the [Angular Commit Message convention][angular commit message]._

[angular commit message]: https://github.com/angular/angular/blob/d684148f93837cf0e4e37146a8df17dc70403558/CONTRIBUTING.md#-commit-message-format

### Opening the Pull Request

From within GitHub, opening a new Pull Request will present you with a
[template] that should be filled out. Please try to do your best at filling out
the details, but feel free to skip parts if you're not sure what to put.

[template]: /.github/PULL_REQUEST_TEMPLATE.md

### Discuss and update

You will probably get feedback or requests for changes to your Pull Request.
This is a big part of the submission process so don't be discouraged!
Some contributors may sign off on the Pull Request right away, others may have
more detailed comments or feedback.
This is a necessary part of the process in order to evaluate whether the changes
are correct and necessary.

**Any community member can review a PR and you might get conflicting feedback**.
Keep an eye out for comments from code owners to provide guidance on conflicting
feedback.

**Once the PR is open, do not rebase the commits**.
See [Commit Squashing](#commit-squashing) for more details.

### Commit Squashing

In most cases, **do not squash commits that you add to your Pull Request during
the review process**.
When the commits in your Pull Request land, they may be squashed into one commit
per logical change.
Metadata will be added to the commit message (including links to the Pull
Request, links to relevant issues, and the names of the reviewers).
The commit history of your Pull Request, however, will stay intact on the Pull
Request page.

## Reviewing Pull Requests

**Any hyperswitch community member is welcome to review any pull request**.

All hyperswitch contributors who choose to review and provide feedback on Pull
Requests have a responsibility to both the project and the individual making the
contribution.
Reviews and feedback must be helpful, insightful, and geared towards improving
the contribution as opposed to simply blocking it.
If there are reasons why you feel the PR should not land, explain what those are.
Do not expect to be able to block a Pull Request from advancing simply because
you say "No" without giving an explanation.
Be open to having your mind changed.
Be open to working with the contributor to make the Pull Request better.

Reviews that are dismissive or disrespectful of the contributor or any other
reviewers are strictly counter to the Code of Conduct.

When reviewing a Pull Request, the primary goals are for the codebase to improve
and for the person submitting the request to succeed.
**Even if a Pull Request does not land, the submitters should come away from the
experience feeling like their effort was not wasted or unappreciated**.
Every Pull Request from a new contributor is an opportunity to grow the
community.

### Review a bit at a time

Do not overwhelm new contributors.

It is tempting to micro-optimize and make everything about relative performance,
perfect grammar, or exact style matches.
Do not succumb to that temptation.

Focus first on the most significant aspects of the change:

1. Does this change make sense for hyperswitch?
2. Does this change make hyperswitch better, even if only incrementally?
3. Are there clear bugs or larger scale issues that need attending to?
4. Is the commit message readable and correct?
   If it contains a breaking change is it clear enough?

Note that only **incremental** improvement is needed to land a PR.
This means that the PR does not need to be perfect, only better than the status
quo.
Follow up PRs may be opened to continue iterating.

When changes are necessary, _request_ them, do not _demand_ them, and **do not
assume that the submitter already knows how to add a test or run a benchmark**.

Specific performance optimization techniques, coding styles and conventions
change over time.
The first impression you give to a new contributor never does.

Nits (requests for small changes that are not essential) are fine, but try to
avoid stalling the Pull Request.
Most nits can typically be fixed by the hyperswitch collaborator landing the
Pull Request but they can also be an opportunity for the contributor to learn a
bit more about the project.

It is always good to clearly indicate nits when you comment: e.g.
`Nit: change foo() to bar(). But this is not blocking.`

If your comments were addressed but were not folded automatically after new
commits or if they proved to be mistaken, please, [hide them][hiding-a-comment]
with the appropriate reason to keep the conversation flow concise and relevant.

### Be aware of the person behind the code

Be aware that _how_ you communicate requests and reviews in your feedback can
have a significant impact on the success of the Pull Request.
Yes, we may land a particular change that makes hyperswitch better, but the
individual might just not want to have anything to do with hyperswitch ever
again.
The goal is not just having good code.

### Abandoned or Stalled Pull Requests

If a Pull Request appears to be abandoned or stalled, it is polite to first
check with the contributor to see if they intend to continue the work before
checking if they would mind if you took it over (especially if it just has nits
left).
When doing so, it is courteous to give the original contributor credit for the
work they started (either by preserving their name and email address in the
commit log, or by using an `Author:` meta-data tag in the commit.

_Adapted from the [Node.js contributing guide][node]_.

[node]: https://github.com/nodejs/node/blob/master/CONTRIBUTING.md
[hiding-a-comment]: https://help.github.com/articles/managing-disruptive-comments/#hiding-a-comment

## Keeping track of issues and PRs

The hyperswitch GitHub repository has a lot of issues and PRs to keep track of.
This section explains the meaning of various labels, as well as our [GitHub
project][project].
The section is primarily targeted at maintainers.
Most contributors aren't able to set these labels.

### Area

The area label describes the area relevant to this issue or PR.

- **A-CI-CD**: This issue/PR concerns our CI/CD setup.
- **A-connector-compatibility**: This issue/PR concerns connector compatibility
  code.
- **A-connector-integration**: This issue/PR concerns connector integrations.
- **A-core**: This issue/PR concerns the core flows.
- **A-dependencies**: The issue/PR concerns one or more of our dependencies.
- **A-drainer**: The issue/PR concerns the drainer code.
- **A-errors**: The issue/PR concerns error messages, error structure or error
  logging.
- **A-framework**: The issue/PR concerns code to interact with other systems or
  services such as database, Redis, connector APIs, etc.
- **A-infra**: This issue/PR concerns deployments, Dockerfiles, Docker Compose
  files, etc.
- **A-macros**: This issue/PR concerns the `router_derive` crate.
- **A-payment-methods**: This issue/PR concerns the integration of new or
  existing payment methods.
- **A-process-tracker**: This issue/PR concerns the process tracker code.

### Category

- **C-bug**: This issue is a bug report or this PR is a bug fix.
- **C-doc**: This issue/PR concerns changes to the documentation.
- **C-feature**: This issue is a feature request or this PR adds new features.
- **C-refactor**: This issue/PR concerns a refactor of existing behavior.
- **C-tracking-issue**: This is a tracking issue for a proposal or for a
  category of bugs.

### Calls for participation

- **E-easy**: This is easy, ranging from quick documentation fixes to stuff you
  can do after getting a basic idea about our product.
- **E-medium**: This is not `E-easy` or `E-hard`.
- **E-hard**: This either involves very tricky code, is something we don't know
  how to solve, or is difficult for some other reason.

### Metadata

The metadata label describes additional metadata that are important for sandbox
or production deployments of our application.

- **M-api-contract-changes**: This PR involves API contract changes.
- **M-configuration-changes**: This PR involves configuration changes.
- **M-database-changes**: This PR involves database schema changes.

### Priority

- **P-low**: This is a low priority issue.
- **P-medium**: This is not `P-low` or `P-high`.
- **P-high**: This is a high priority issue and must be addressed quickly.

### RFCs

- **RFC-in-progress**: This RFC involves active discussion regarding substantial
  design changes.
- **RFC-resolved**: This RFC has been resolved.

### Status

The status label provides information about the status of the issue or PR.

- **S-awaiting-triage**: This issue or PR is relatively new and has not been
  addressed yet.
- **S-blocked**: This issue or PR is blocked on something else, or other
  implementation work.
- **S-design**: This issue or PR involves a problem without an obvious solution;
  or the proposed solution raises other questions.
- **S-in-progress**: The implementation relevant to this issue/PR is underway.
- **S-invalid**: This is an invalid issue.
- **S-needs-conflict-resolution**: This PR requires merge conflicts to be
  resolved by the author.
- **S-needs-reproduction-steps**: This behavior hasn't been reproduced by the
  team.
- **S-unactionable**: There is not enough information to act on this problem.
- **S-unassigned**: This issue has no one assigned to address it.
- **S-waiting-on-author**: This PR is incomplete or the author needs to address
  review comments.
- **S-waiting-on-reporter**: Awaiting response from the issue author.
- **S-waiting-on-review**: This PR has been implemented and needs to be reviewed.
- **S-wont-fix**: The proposal in this issue was rejected and will not be
  implemented.

Any label not listed here is not in active use.

[project]: https://github.com/orgs/juspay/projects/3
