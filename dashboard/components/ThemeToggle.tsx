import React from 'react';
import { useTheme } from '../contexts/ThemeContext';

export const ThemeToggle: React.FC = () => {
  const { theme, toggleTheme } = useTheme();
  const label = theme === 'dark' ? 'Light Mode' : 'Dark Mode';
  return (
    <button
      onClick={toggleTheme}
      aria-label="Toggle dark mode"
      className="p-1 border rounded-md text-sm bg-white dark:bg-gray-800"
    >
      {label}
    </button>
  );
};
