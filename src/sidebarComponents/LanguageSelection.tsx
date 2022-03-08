import { Select } from "@chakra-ui/select";
import "react";
import { useCustomToasts } from "../useCustomToasts";
import { Language, languages } from "../languages";

type LanguageSelectionProps = {
  language: Language;
  setLanguage: (newLanguage: Language) => unknown;
  darkMode: boolean;
};

export function LanguageSelection({
  language,
  setLanguage,
  darkMode,
}: LanguageSelectionProps) {
  const toasts = useCustomToasts();
  function handleChangeLanguage(language: Language) {
    setLanguage(language);
    // TODO: Clear up whether its on purpose that the toast is only shown for the user changing the language,
    // TODO: then figure out how to coordinate the setLanguage for the server with rustpad.setLanguage
    // if (rustpad.current?.setLanguage(language)) {
    toasts.languageChange(language);
    // }
  }
  return (
    <Select
      size="sm"
      bgColor={darkMode ? "#3c3c3c" : "white"}
      borderColor={darkMode ? "#3c3c3c" : "white"}
      value={language}
      onChange={(event) => handleChangeLanguage(event.target.value as Language)}
    >
      {languages.map((lang) => (
        <option key={lang} value={lang} style={{ color: "black" }}>
          {lang}
        </option>
      ))}
    </Select>
  );
}
