import { useCallback } from 'react';

export const useDayTabSelection = (setSelectedDay: (day: string) => void) => {
  return useCallback(
    (day: number) => {
      const safeDay = Number.isFinite(day) ? Math.max(1, Math.floor(day)) : 1;
      const dayStr = safeDay >= 10 ? 'Day 10+' : `Day ${safeDay}`;
      setSelectedDay(dayStr);
    },
    [setSelectedDay],
  );
};
