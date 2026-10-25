"""``.github/dependabot.yml`` is read by a service, not by CI.

Dependabot validates the file against its hosted schema and reports a rejection as a check-run on the
commit that carried it, while every workflow keeps passing: the failure mode is that an entry stops
scheduling the updates it exists for, not that something goes red. Two vocabularies live in this file
and each is valid in exactly one place - measured rather than inferred, because the file merged
carrying the prefixed form under ``groups`` and Dependabot answered with five rejections naming
``major, minor, patch`` as the only values accepted there.

* ``groups.<name>.update-types`` -> ``major`` / ``minor`` / ``patch``
* ``ignore.<dependency>.update-types``, like ``allow`` -> ``version-update:semver-*``

The two assertions are separate on purpose: an edit that "makes the file consistent" reddens one of
them whichever direction it goes, and each test also fails if the sites it is supposed to police
disappear, so neither can go green by measuring nothing.
"""

from __future__ import annotations

from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
CONFIG = REPO_ROOT / ".github" / "dependabot.yml"

GROUP_VOCABULARY = frozenset({"major", "minor", "patch"})
SEMVER_VOCABULARY = frozenset(
    {
        "version-update:semver-major",
        "version-update:semver-minor",
        "version-update:semver-patch",
    }
)


def _updates() -> list[dict]:
    return yaml.safe_load(CONFIG.read_text(encoding="utf-8"))["updates"]


def _group_declared_types() -> dict[str, list[str]]:
    declared: dict[str, list[str]] = {}
    for entry in _updates():
        for name, group in (entry.get("groups") or {}).items():
            update_types = group.get("update-types")
            if update_types:
                declared[f"{entry['package-ecosystem']}/{name}"] = list(update_types)
    return declared


def _ignore_declared_types() -> dict[str, list[str]]:
    declared: dict[str, list[str]] = {}
    for entry in _updates():
        for rule in entry.get("ignore") or []:
            update_types = rule.get("update-types")
            if update_types:
                declared[str(rule.get("dependency-name"))] = list(update_types)
    return declared


def test_group_update_types_use_the_short_vocabulary() -> None:
    declared = _group_declared_types()
    assert declared, (
        "no dependabot group declares update-types any more, so this check has nothing to read - "
        "delete it with the groups, do not let it pass in their absence"
    )
    offenders = {
        name: sorted(set(values) - GROUP_VOCABULARY)
        for name, values in declared.items()
        if set(values) - GROUP_VOCABULARY
    }
    assert not offenders, (
        "groups.*.update-types accepts only major/minor/patch; a prefixed value there is rejected by "
        f"the hosted schema and the group silently stops offering updates: {offenders}"
    )


def test_ignore_update_types_use_the_semver_vocabulary() -> None:
    declared = _ignore_declared_types()
    assert declared, "no dependabot ignore rule declares update-types any more, so this check has nothing to read"
    offenders = {
        name: sorted(set(values) - SEMVER_VOCABULARY)
        for name, values in declared.items()
        if set(values) - SEMVER_VOCABULARY
    }
    assert not offenders, (
        "ignore.*.update-types is spelled in the version-update vocabulary; the short forms the "
        f"groups need are not accepted here: {offenders}"
    )


def test_the_two_vocabularies_are_still_distinct_in_the_file() -> None:
    """The pair above only bites while the same file really uses both spellings.

    If one vocabulary ever vanishes from the config, the matching test's own emptiness guard fires;
    this assertion is the earlier, cheaper statement of the same fact and names the asymmetry as the
    thing under test rather than leaving it implicit in two passes.
    """
    assert _group_declared_types(), "groups side missing"
    assert _ignore_declared_types(), "ignore side missing"
