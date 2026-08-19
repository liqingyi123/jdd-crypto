import prettier from "prettier/standalone";
import parserBabel from "prettier/plugins/babel";
import parserEstree from "prettier/plugins/estree";

async function formatWith(
  str: string,
  parser: "json" | "babel",
  tabWidth: number,
): Promise<string> {
  return prettier.format(str, {
    parser,
    plugins: [parserBabel, parserEstree],
    tabWidth,
    // Expand nested structures so Monaco folding controls are useful.
    printWidth: 1,
    semi: false,
    singleQuote: false,
    trailingComma: "none",
  });
}

/** Try to format with Prettier; return original text when parsing fails. */
export async function prettierFormat(
  str: string,
  tabWidth = 2,
): Promise<string> {
  if (!str.trim()) {
    return str;
  }
  try {
    return await formatWith(str, "json", tabWidth);
  } catch {
    // fall through
  }

  const trimmed = str.trim();
  // Avoid babel rewriting plain key:value / ciphertext lines.
  if (
    trimmed.startsWith("{") ||
    trimmed.startsWith("[") ||
    trimmed.startsWith("(")
  ) {
    try {
      return await formatWith(str, "babel", tabWidth);
    } catch {
      return str;
    }
  }
  return str;
}
