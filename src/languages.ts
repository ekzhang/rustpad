export const languages = [
  "abap",
  "aes",
  "apex",
  "azcli",
  "bat",
  "bicep",
  "c",
  "cameligo",
  "clojure",
  "coffeescript",
  "cpp",
  "csharp",
  "csp",
  "css",
  "dart",
  "dockerfile",
  "ecl",
  "elixir",
  "flow9",
  "fsharp",
  "go",
  "graphql",
  "handlebars",
  "hcl",
  "html",
  "ini",
  "java",
  "javascript",
  "json",
  "julia",
  "kotlin",
  "less",
  "lexon",
  "liquid",
  "lua",
  "m3",
  "markdown",
  "mips",
  "msdax",
  "mysql",
  "objective-c",
  "pascal",
  "pascaligo",
  "perl",
  "pgsql",
  "php",
  "pla",
  "plaintext",
  "postiats",
  "powerquery",
  "powershell",
  "proto",
  "pug",
  "python",
  "qsharp",
  "r",
  "razor",
  "redis",
  "redshift",
  "restructuredtext",
  "ruby",
  "rust",
  "sb",
  "scala",
  "scheme",
  "scss",
  "shell",
  "sol",
  "sparql",
  "sql",
  "st",
  "swift",
  "systemverilog",
  "tcl",
  "twig",
  "typescript",
  "vb",
  "verilog",
  "xml",
  "yaml",
] as const;

export type Language = typeof languages[number];

type LanguageExtensionDecls = {
  /** the default extension (for download and to recognize uploads)*/
  extension: string,
  /** Other known extensions for that language (to detect it in uploads etc) */
  aliasExtensions: string[],
};

const languagesAndExtensions: {[key in Language]?: LanguageExtensionDecls} = {
  typescript: {
    extension: "ts",
    aliasExtensions: ["tsx"],
  },
  rust: {
    extension: "rs",
    aliasExtensions: []
  },
  cpp: {
    extension: "cpp",
    aliasExtensions: ["cxx"],
  },
  yaml: {
    extension: "yaml",
    aliasExtensions:["yml"],
  },
  html: {
    extension: "html",
    aliasExtensions: ["htm"],
  },
};

type LanguageExtensions = { [key in string]: Language };

/** Used on download to determine file extension. */
export function getFileExtension(language: Language) {
  return languagesAndExtensions[language as keyof typeof languagesAndExtensions]?.extension ?? "txt";
}

const defaultExtensionsToLanguage = {} as LanguageExtensions;

const additionalExtensionsToLanguage = {} as LanguageExtensions;

Object.entries(languagesAndExtensions).forEach(([language, {extension, aliasExtensions}]) => {
  defaultExtensionsToLanguage[extension] = language as Language;
  aliasExtensions.forEach((aliasExtension) => {
    additionalExtensionsToLanguage[aliasExtension] = language as Language;
  })
})

const extensionsToLanguage: LanguageExtensions = {
  ...additionalExtensionsToLanguage,
  ...defaultExtensionsToLanguage,
};

/** Used to detect programming language on upload based on file name */
export function getLanguage(fileName: string): Language {
  return extensionsToLanguage[fileName.split(".")[1]] ?? "plaintext";
}
