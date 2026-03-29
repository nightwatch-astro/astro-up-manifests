#!/usr/bin/env python3
"""Convert old Go-format manifests to new Rust-format TOML manifests."""

import os
import sys
import tomllib

OLD_DIR = sys.argv[1] if len(sys.argv) > 1 else "/Users/sjors/personal/dev/astro-up/repos/astro-up-manifests/manifests"
NEW_DIR = sys.argv[2] if len(sys.argv) > 2 else "manifests"

INSTALL_METHOD_MAP = {
    "innosetup": "inno_setup",
    "inno_setup": "inno_setup",
    "msi": "msi",
    "nsis": "nsis",
    "zip": "zip_wrap",
    "zip_wrap": "zip_wrap",
    "exe": "exe",
    "download": "download_only",
    "download_only": "download_only",
}

DETECTION_METHOD_MAP = {
    "registry": "registry",
    "file_exists": "file",
    "file": "file",
    "pe_file": "pe_file",
    "directory": "directory",
}


def map_provider(remote):
    provider = remote.get("provider", "")
    version_check = remote.get("version_check", "")
    if provider == "github":
        return "github"
    elif provider == "gitlab":
        return "gitlab"
    elif version_check in ("go_scrape", "html_scrape"):
        return "html_scrape"
    elif version_check in ("rod_scrape", "browser_scrape"):
        return "browser_scrape"
    elif version_check == "pe_download":
        return "pe_download"
    elif version_check == "direct_url":
        return "direct_url"
    elif version_check == "http_head":
        return "http_head"
    return "manual"


def q(s):
    """Quote a string for TOML. Escapes backslashes in basic strings."""
    if "\\" in s:
        escaped = s.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    if '"' in s:
        return f"'{s}'"
    return f'"{s}"'


def qa(items):
    """Quote a list of strings for TOML array."""
    return "[" + ", ".join(q(i) for i in items) + "]"


def convert_manifest(old_path, new_path, category_from_dir):
    with open(old_path, "rb") as f:
        old = tomllib.load(f)

    pkg_id = old.get("id", "")
    if not pkg_id:
        print(f"  SKIP (no id): {old_path}")
        return False

    with open(new_path, "w") as f:
        f.write(f"manifest_version = 1\n")
        f.write(f"id = {q(pkg_id)}\n")
        f.write(f"name = {q(old.get('name', pkg_id))}\n")

        for key in ["description", "publisher", "homepage"]:
            val = old.get(key, "")
            if val:
                f.write(f"{key} = {q(val)}\n")

        f.write(f"category = {q(old.get('category', category_from_dir))}\n")
        f.write(f"type = {q(old.get('type', 'application'))}\n")
        f.write(f"slug = {q(old.get('slug', pkg_id))}\n")

        for key in ["tags", "aliases"]:
            val = old.get(key, [])
            if val:
                f.write(f"{key} = {qa(val)}\n")

        if old.get("license"):
            f.write(f"license = {q(old['license'])}\n")

        # Detection
        detection = old.get("detection", {})
        if detection:
            f.write(f"\n[detection]\n")
            method = DETECTION_METHOD_MAP.get(detection.get("method", ""), detection.get("method", ""))
            f.write(f"method = {q(method)}\n")
            if "file_path" in detection:
                f.write(f"path = {q(detection['file_path'])}\n")
            if "registry_key" in detection:
                f.write(f"registry_key = {q(detection['registry_key'])}\n")
            if "registry_value" in detection:
                f.write(f"registry_value = {q(detection['registry_value'])}\n")
            if detection.get("file_version"):
                f.write(f"file_version = true\n")

            fallback = detection.get("fallback", {})
            if fallback:
                fb_method = DETECTION_METHOD_MAP.get(fallback.get("method", ""), fallback.get("method", ""))
                f.write(f"fallback_method = {q(fb_method)}\n")
                if "file_path" in fallback:
                    f.write(f"fallback_path = {q(fallback['file_path'])}\n")

        # Install
        install = old.get("install", {})
        f.write(f"\n[install]\n")
        method = INSTALL_METHOD_MAP.get(install.get("method", "exe"), install.get("method", "exe"))
        f.write(f"method = {q(method)}\n")
        if install.get("scope"):
            f.write(f"scope = {q(install['scope'])}\n")
        if install.get("elevation", method in ("msi", "exe")):
            f.write(f"elevation = true\n")

        quiet_args = install.get("quiet_args", [])
        if quiet_args:
            args_str = " ".join(quiet_args)
            f.write(f"\n[install.switches]\n")
            f.write(f"silent = {q(args_str)}\n")

        # Checkver (from [remote])
        remote = old.get("remote", {})
        if remote:
            provider = map_provider(remote)
            f.write(f"\n[checkver]\n")
            f.write(f"provider = {q(provider)}\n")

            if provider in ("github", "gitlab"):
                if "owner" in remote:
                    f.write(f"owner = {q(remote['owner'])}\n")
                if "repo" in remote:
                    f.write(f"repo = {q(remote['repo'])}\n")

            scrape_url = remote.get("scrape_url", remote.get("url", remote.get("download_page", "")))
            if scrape_url and provider in ("html_scrape", "browser_scrape", "direct_url", "http_head", "pe_download"):
                f.write(f"url = {q(scrape_url)}\n")

            scrape_regex = remote.get("scrape_regex", remote.get("regex", ""))
            if scrape_regex:
                # Use single-quoted literal string for regex (no escape processing)
                f.write(f"regex = '{scrape_regex}'\n")

            download_url = remote.get("download_url", remote.get("url", ""))
            if download_url and provider not in ("github", "gitlab"):
                f.write(f"\n[checkver.autoupdate]\n")
                f.write(f"url = {q(download_url)}\n")

        # Dependencies
        requires = old.get("requires", [])
        if requires:
            f.write(f"\n[dependencies]\n")
            f.write(f"requires = {qa(requires)}\n")

        # Hardware
        hardware = old.get("hardware", {})
        if hardware:
            f.write(f"\n[hardware]\n")
            for key in ["device_class", "inf_provider"]:
                if key in hardware:
                    f.write(f"{key} = {q(hardware[key])}\n")
            if hardware.get("vid_pid"):
                f.write(f"vid_pid = {qa(hardware['vid_pid'])}\n")

        # Backup
        backup = old.get("backup", {})
        config_paths = backup.get("config_paths", old.get("config_paths", []))
        if config_paths:
            f.write(f"\n[backup]\n")
            f.write(f"config_paths = {qa(config_paths)}\n")

    return True


def main():
    os.makedirs(NEW_DIR, exist_ok=True)
    converted = skipped = errors = 0

    for category_dir in sorted(os.listdir(OLD_DIR)):
        category_path = os.path.join(OLD_DIR, category_dir)
        if not os.path.isdir(category_path):
            continue
        for filename in sorted(os.listdir(category_path)):
            if not filename.endswith(".toml"):
                continue
            old_path = os.path.join(category_path, filename)
            try:
                with open(old_path, "rb") as f:
                    old = tomllib.load(f)
                pkg_id = old.get("id", filename.replace(".toml", ""))
                new_path = os.path.join(NEW_DIR, f"{pkg_id}.toml")
                if os.path.exists(new_path):
                    print(f"  EXISTS: {new_path}")
                    skipped += 1
                    continue
                if convert_manifest(old_path, new_path, category_dir):
                    print(f"  OK: {pkg_id}")
                    converted += 1
                else:
                    skipped += 1
            except Exception as e:
                print(f"  ERROR: {old_path}: {e}")
                errors += 1

    print(f"\nDone: {converted} converted, {skipped} skipped, {errors} errors")


if __name__ == "__main__":
    main()
