-- Which end of the resolved timesheet period ('start' or 'end') is used for the date
-- in a generated timesheet's filename. Defaults to 'end' (the last day of the
-- week/month), matching the common last-day-of-period filename convention.
ALTER TABLE contracts ADD COLUMN filename_date TEXT NOT NULL DEFAULT 'end';
