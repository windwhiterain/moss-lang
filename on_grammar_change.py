import subprocess
import shutil

subprocess.run(["tree-sitter", "generate"], cwd=r"tree-sitter-moss", check=True)
subprocess.run(["type-sitter-cli", "tree-sitter-moss/src/node-types.json"], check=True)
shutil.copy2(
    r"src/type_sitter/moss.rs", r"interpreter/src/type_sitter_lang/moss/moss_gen.rs"
)
shutil.rmtree(r"src")
