<div align="center">

```text
              _   _        _   _      _ _     _   _           _ _    
 __ _ ___| |_| |_ __ _| |_| |_ __| (_)___| |_| |__  _   _| | | __
/ _` / -_)  _|  _/ _` |  _|  _/ _` | / __|  _| '_ \| || | | |/ /
\__, \___|\__|\__\__,_|\__|\__\__,_|_\___|\__|_.__/ \_,_|_|_|\_\
|___/                                                           
```

**Safe Rust bindings for the macOS `getattrlistbulk()` system call.** <br>
*Enumerate directories and retrieve file metadata in bulk with minimal syscalls.*

[![Crates.io](https://img.shields.io/crates/v/getattrlistbulk?style=for-the-badge)](https://crates.io/crates/getattrlistbulk)
[![Docs.rs](https://img.shields.io/docsrs/getattrlistbulk?style=for-the-badge)](https://docs.rs/getattrlistbulk)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=for-the-badge&logo=rust)](https://rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-macOS-lightgrey?style=for-the-badge&logo=apple)](https://apple.com)
[![License](https://img.shields.io/crates/l/getattrlistbulk?style=for-the-badge)](https://github.com/quivent/getattrlistbulk-rs)

</div>

---

## 📖 Table of Contents

- [Why getattrlistbulk?](#-why-getattrlistbulk)
- [Requirements](#-requirements)
- [Features](#-features)
- [Installation](#-installation)
- [Usage](#-usage)
  - [Basic Example](#basic-example)
  - [Get All Available Metadata](#get-all-available-metadata)
- [Contributing](#-contributing)
- [License](#-license)

---

## ⚡ Why getattrlistbulk?

Traditional directory reading on macOS requires `N+1` syscalls for `N` files. When working with large directories, this I/O overhead can become a significant bottleneck.

`getattrlistbulk()` drastically reduces this overhead by retrieving entries AND metadata together in bulk batches.

### Performance Comparison

| Operation Pattern | Call Sequence | Syscalls (for 10,000 files) |
| :--- | :--- | :--- |
| **Traditional** | `opendir()` → `readdir()` × N → `stat()` × N → `closedir()` | **~20,000** |
| **getattrlistbulk** | `open()` → `getattrlistbulk()` × ceil(N/batch) → `close()` | **~10** |

---

## ⚠️ Requirements

> [!IMPORTANT]
> **Platform Restriction**
> This crate relies on a macOS-specific system call and only compiles on macOS. On other platforms, it will fail to compile with a clear error message.

- **OS:** macOS 10.10+ (Yosemite or later)
- **Rust:** 1.70+

---

## ✨ Features

- **Blazing Fast:** Minimizes context switches by fetching directory entries and metadata in batches.
- **Strongly Typed:** Safe, ergonomic Rust interface over complex C structs and buffer management.
- **Granular Control:** Request only the specific file attributes you need to further optimize performance.
- **Zero-Cost Abstractions:** Minimal overhead above the underlying system calls.

---

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
getattrlistbulk = "0.1"
```

---

## 🚀 Usage

### Basic Example

Enumerate a directory and print the sizes of all files, requesting only the specific metadata we need.

```rust
use getattrlistbulk::{read_dir, RequestedAttributes};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Specify only the attributes you need
    let attrs = RequestedAttributes {
        name: true,
        size: true,
        object_type: true,
        ..Default::default()
    };

    // Iterate through the directory
    for entry in read_dir("/Users/me/Documents", attrs)? {
        let entry = entry?;
        println!("{}: {} bytes", entry.name, entry.size.unwrap_or(0));
    }

    Ok(())
}
```

<details>
<summary><strong>View all available metadata fields</strong></summary>

### Get All Available Metadata

You can request a comprehensive set of metadata fields if your application requires it:

```rust
use getattrlistbulk::{read_dir, RequestedAttributes};

let attrs = RequestedAttributes {
    name: true,
    object_type: true,
    size: true,
    alloc_size: true,
    modified_time: true,
    permissions: true,
    inode: true,
    entry_count: true,  // specifically for directories
};

for entry in read_dir("/path/to/dir", attrs)? {
    let entry = entry?;
    
    if let Some(perms) = entry.permissions {
        // Handle permissions
    }
    if let Some(mtime) = entry.modified_time {
        // Handle modification time
    }
}
```
</details>

---

## 🤝 Contributing

Contributions are always welcome! If you've found a bug or have a feature request, please open an issue.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

This project is dual-licensed under the MIT and Apache-2.0 licenses. 
See the `LICENSE-MIT` and `LICENSE-APACHE` files for more details.
