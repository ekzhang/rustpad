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

/** The default extension of the known programming languages (to be used on download) */
const defaultExtensions: { [key in Language]?: string } = {
  typescript: "ts",
  rust: "rs",
  cpp: "cpp",
  yaml: "yaml",
  html: "html",
};

type LanguageExtensions = { [key in string]: Language };

/**
 * Additional extensions that refer to known languages, to detect filetype on upload.
 * NO NEED to add the default file extensions.
 */
const additionalExtensionsToLanguage: LanguageExtensions = {
  yml: "yaml",
  cc: "cpp",
};

/** Used on download to determine file extension. */
export function getFileExtension(language: Language) {
  return defaultExtensions[language] ?? "txt";
}

/** invert key value */
const defaultExtensionsToLanguage = Object.entries(defaultExtensions).reduce(
  (defaultExtensionsToLanguage, [key, value]) => {
    return {
      ...defaultExtensionsToLanguage,
      [value]: key as Language,
    };
  },
  {} as LanguageExtensions
);

const extensionsToLanguage: LanguageExtensions = {
  ...defaultExtensionsToLanguage,
  ...additionalExtensionsToLanguage,
};

/** Used to detect programming language on upload based on file name */
export function getLanguage(fileName: string): Language {
  return extensionsToLanguage[fileName.split(".")[1]] ?? "plaintext";
}
