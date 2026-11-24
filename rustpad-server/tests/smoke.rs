//! Smoke tests against a real go-judge runner. Run them with
//! `compose/smoke-test.sh`; every test is ignored by default and reads
//! `GO_JUDGE_URL` and `GO_JUDGE_TOKEN`.

use std::time::Instant;

use rustpad_server::runner::{ExecOutput, GoJudgeClient, RunStage};

const LANGUAGES: [&str; 6] = ["python", "c", "cpp", "java", "kotlin", "javascript"];

fn client() -> GoJudgeClient {
    let _ = pretty_env_logger::try_init();
    let url = std::env::var("GO_JUDGE_URL").expect("GO_JUDGE_URL must be set");
    let token = std::env::var("GO_JUDGE_TOKEN")
        .ok()
        .filter(|token| !token.is_empty());
    GoJudgeClient::new(&url, token).unwrap()
}

/// Runs `source` and prints the wall time.
async fn execute(language: &str, source: &str) -> ExecOutput {
    let start = Instant::now();
    let output = client().execute(language, source).await.unwrap();
    println!("{language}: {} ms", start.elapsed().as_millis());
    output
}

/// Runs `source`, checks that it compiled, and returns the run stage.
async fn run(language: &str, source: &str) -> RunStage {
    let output = execute(language, source).await;
    if let Some(compile) = &output.compile {
        assert_eq!(compile.code, Some(0), "compile failed: {compile:?}");
    }
    output.run.expect("no run stage")
}

async fn assert_prints(language: &str, source: &str, expected: &str) {
    let stage = run(language, source).await;
    assert_eq!(stage.stdout, expected, "{stage:?}");
    assert_eq!(stage.code, Some(0), "{stage:?}");
}

#[tokio::test]
#[ignore]
async fn smoke_probes() {
    let client = client();
    for language in LANGUAGES {
        let version = client.probe(language).await.unwrap();
        println!("{language}: {version}");
    }
}

#[tokio::test]
#[ignore]
async fn smoke_hello_python() {
    assert_prints(
        "python",
        "print(\"hello from python\")\n",
        "hello from python\n",
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn smoke_hello_c() {
    let source = "#include <stdio.h>\n\
                  int main(void) { printf(\"hello from c\\n\"); return 0; }\n";
    assert_prints("c", source, "hello from c\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_hello_cpp() {
    let source = "#include <iostream>\n\
                  int main() { std::cout << \"hello from cpp\\n\"; }\n";
    assert_prints("cpp", source, "hello from cpp\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_hello_java() {
    let source = "public class Main {\n\
                  public static void main(String[] args) {\n\
                  System.out.println(\"hello from java\");\n\
                  }\n\
                  }\n";
    assert_prints("java", source, "hello from java\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_hello_kotlin() {
    let source = "fun main() {\n    println(\"hello from kotlin\")\n}\n";
    assert_prints("kotlin", source, "hello from kotlin\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_hello_javascript() {
    let source = "console.log(\"hello from javascript\");\n";
    assert_prints("javascript", source, "hello from javascript\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_java_solution() {
    let source = r#"package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public class Solution {
    public static void main(String[] args) {
        List<Integer> numbers = new ArrayList<>(List.of(5, 3, 9, 1, 7));
        Collections.sort(numbers);
        System.out.println(numbers);
    }
}
"#;
    assert_prints("java", source, "[1, 3, 5, 7, 9]\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_kotlin_program() {
    let source = r#"data class Person(val name: String, val age: Int)

fun main() {
    val people = listOf(Person("Ana", 34), Person("Bo", 17), Person("Cy", 52))
    val adults = people.filter { it.age >= 18 }.map { "${it.name} (${it.age})" }
    println("Adults: ${adults.joinToString()}")
    println(people.first())
}
"#;
    let expected = "Adults: Ana (34), Cy (52)\nPerson(name=Ana, age=34)\n";
    assert_prints("kotlin", source, expected).await;
}

#[tokio::test]
#[ignore]
async fn smoke_clean_workdir() {
    let source = "for path in (\"/w/leftover\", \"/tmp/leftover\"):\n\
                  \x20   open(path, \"w\").write(\"x\")\n\
                  print(\"wrote\")\n";
    assert_prints("python", source, "wrote\n").await;
    let source = "import os\n\
                  left = [p for p in (\"/w/leftover\", \"/tmp/leftover\") if os.path.exists(p)]\n\
                  print(left or \"clean\")\n";
    assert_prints("python", source, "clean\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_stdin_is_empty() {
    let source = "try:\n    input()\nexcept EOFError:\n    print(\"eof\")\n";
    assert_prints("python", source, "eof\n").await;
}

#[tokio::test]
#[ignore]
async fn smoke_limits() {
    let stage = run("python", "while True: pass\n").await;
    assert_eq!(
        stage.status.as_deref(),
        Some("Time limit exceeded"),
        "{stage:?}"
    );

    let stage = run("python", "x = bytearray(1 << 30)\n").await;
    assert_eq!(
        stage.status.as_deref(),
        Some("Memory limit exceeded"),
        "{stage:?}"
    );

    let source = "import socket\nsocket.create_connection((\"1.1.1.1\", 80), timeout=2)\n";
    let stage = run("python", source).await;
    assert_ne!(stage.code, Some(0), "{stage:?}");

    let source = "with open(\"/w/big\", \"wb\") as f:\n\
                  \x20   for _ in range(200):\n\
                  \x20       f.write(b\"x\" * (1 << 20))\n";
    let stage = run("python", source).await;
    assert_ne!(stage.code, Some(0), "{stage:?}");
    assert_prints(
        "python",
        "print(\"hello from python\")\n",
        "hello from python\n",
    )
    .await;

    let stage = run("python", "print(\"x\" * 10_000_000)\n").await;
    assert!(stage.truncated, "{stage:?}");
}
