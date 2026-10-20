Add a total hours to contracts. Calculate remaining hours based off of time entries.
List contracts without time on reports page.
EXE icon is blank when on task bar
Fix Today to show only today and not future entries.
Create an email template that states what Companies have no time.
App content should fill the full window width. Fields/forms stop widening at a
fixed point and leave empty space on wide windows. Likely causes: `main { max-width: 960px }`
in src/App.svelte:155, and the `max-width: 32rem` (40rem on ImportPanel) caps on the
panel forms (AdminPanel, AppearancePanel, BackupPanel, UserPanel, ImportPanel,
ImportTemplateDetail, EmailTemplateDetail).
