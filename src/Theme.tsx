import { IconButton, Tooltip } from "@chakra-ui/react";
import React, { useContext } from "react";
import { FaSun, FaMoon } from "react-icons/fa";


export const ThemeContext = React.createContext({
  darkMode: false,
  toggleDarkMode: () => { }
})

export function DarkModeToggle() {
  const { darkMode, toggleDarkMode } = useContext(ThemeContext)
  return (
    <Tooltip label="Toggle dark mode">
      <IconButton
        aria-label="Toggle dark mode"
        size="xs"
        rounded="none"
        onClick={toggleDarkMode}
        bgColor={darkMode ? "#333333" : "#e8e8e8"}
        _hover={{ bg: darkMode ? "#575757" : "gray.200" }}
        icon={darkMode ? <FaSun /> : <FaMoon />}
      />
    </Tooltip>
  )
}
