# fungus-cli

FUNGUS is a tool for detecting similarities between ARMv7 assembly projects, for example, for introductory software assignments. This is the command-line tool which performs the analysis and generates a plagiarism report in JSON format. It is meant to be used in conjunction with a desktop GUI, such as [fungus-gui](https://github.com/Project-Fungus/fungus-gui).

FUNGUS is inspired by Stanford's [Measure of Software Similarity](https://theory.stanford.edu/~aiken/moss/) (Moss). At its core, it uses the same algorithm, winnowing, described in [this paper](https://theory.stanford.edu/~aiken/publications/papers/sigmod03.pdf).

## Installation

### Binary

1. Go to the [Releases page](https://github.com/Project-Fungus/fungus-cli/releases).
2. Download the artifact for your platform.
3. Add the command to your `PATH`.

Run `fungus --version` to check that the installation was successful.

### Building From Source

1. Clone this repository or download the code.
2. Ensure you have [installed Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).
3. Run `cargo build --release`. The binary will be placed in the `target/release/` directory.

## Key Inputs

### Root

FUNGUS assumes the projects to analyze are all in separate directories, each a direct child of the same root directory. For example, consider the following directory structure:

```
submissions/
├── project1
│   ├── subdir1
│   │   └── file1.s
│   └── subdir2
│       └── file2.s
├── project2
│   ├── code1.s
│   └── code2.s
└── starter-code
    ├── file1.s
    └── file2.s
```

If the `submissions/` directory is selected as the root, then FUNGUS will select `project1`, `project2`, and `starter-code` as the projects to compare.

### Starter Code

Paths to ignore (e.g., assignment starter code provided to all students) can be given as input to FUNGUS. Any code in students' projects that match this code will not be flagged as potential plagiarism. The paths to ignore can be inside the root directory (as in the example above) or outside of it.

### Tokenizer

Two tokenizers are available:
- The "naive" tokenizer is a straightforward, best-effort lexer for GNU ARMv7 assembly. In some cases, it may incorrectly identify tokens (e.g., if a student names a label `r10`).
- The "relative" tokenizer is a more conservative lexer that identifies some tokens by the *distance to their most recent occurrence*. This implicitly handles most cases of register and label renaming.

### Noise Threshold, Guarantee Threshold, and Max Token Offset

FUNGUS accepts noise and guarantee thresholds as inputs.
- The noise threshold defines a minimum size for matches to be reported at all. Matching code snippets that have fewer than this number of tokens will not be flagged as potential plagiarism.
- The guarantee threshold defines a minimum size beyond which matches are *guaranteed* to be reported. That is, if two project have matching code snippets that are at least as long as the guarantee threshold, then that match will always be flagged as plagiarism.

In addition, when using the "relative" tokenizer, an additional max token offset can be specified. This is the maximum value of the distance for relative tokens. Intuitively, choosing a very small max offset will probably result in many false positives. In the extreme case of the max offset being 0, this reduces to non-relative lexing but with no distinction between registers, labels, etc. Conversely, choosing a very large max offset will probably result in many false negatives. In the extreme case of there being no limit, the results depend on the overall structure of the document. In that case, there is no guarantee that any matches will be reported (unless two files are identical).

## Output Format

```json
{
	"warnings": [
		{
			"file": "project1/my_invalid_file.s",
			"message": "Message explaining what's wrong.",
			"warn_type": "Type"
		}
	],
	"project_pairs": [
		{
			"project1": "Project 1",
			"project2": "Project 2",
			"matches": [
				{
					"project_1_location": {
						"file": "Project 1/code.s",
						"span": {
							"start": 0,
							"end": 42
						}
					},
					"project_2_location": {
						"file": "Project 2/my_code.s",
						"span": {
							"start": 100,
							"end": 150
						}
					}
				}
			]
		}
	]
}
```

Note that:
- In the `warnings` field:
	- The file is optional. For example, there may be warnings about the arguments chosen for this analysis.
	- Valid values for the `warn_type` include "Args," "Input," and "Fingerprint." See the `WarningType` enum for the full list.
- In the `project_pairs` field:
	- All file paths are relative to the `root` argument.
	- For each `span`:
		- The start and end values are bytes (not necessarily characters!).
		- The start value is inclusive.
		- The end value is exclusive.
