### Clipboard Manager: every text of the window, in English.
### English is also the fallback: a text missing in another language shows in
### English. Keys say what a text is for, starting with the part of the window
### it belongs to. docs/TRANSLATING.md explains how to add a language.

## Times and dates

# A 12-hour clock time, such as "2:05 PM".
time-clock-am = { $time } AM
time-clock-pm = { $time } PM
time-just-now = just now
time-minutes-ago =
    { $count ->
        [one] 1 minute ago
       *[other] { $count } minutes ago
    }
time-hours-ago =
    { $count ->
        [one] 1 hour ago
       *[other] { $count } hours ago
    }
time-yesterday = yesterday
time-today-at = Today at { $time }
time-yesterday-at = Yesterday at { $time }
# $date is a whole date, such as "Mon 14 Oct 2026".
time-date-at = { $date } at { $time }
date-day-month = { $day } { $month }
date-day-month-year = { $day } { $month } { $year }
date-weekday-date = { $weekday } { $date }
weekday-mon = Mon
weekday-tue = Tue
weekday-wed = Wed
weekday-thu = Thu
weekday-fri = Fri
weekday-sat = Sat
weekday-sun = Sun
month-jan = Jan
month-feb = Feb
month-mar = Mar
month-apr = Apr
month-may = May
month-jun = Jun
month-jul = Jul
month-aug = Aug
month-sep = Sep
month-oct = Oct
month-nov = Nov
month-dec = Dec

## Day headers in the list, shown in capitals

group-pinned = Pinned
group-today = Today
group-yesterday = Yesterday
group-this-week = This week
group-this-month = This month
group-older = Older
