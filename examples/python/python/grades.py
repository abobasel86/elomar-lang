"""Helpers called from main.elo through std.py."""

import statistics


def describe(grades, course):
    return {
        "course": course,
        "mean": round(statistics.mean(grades), 2),
        "median": statistics.median(grades),
        "stdev": round(statistics.stdev(grades), 2),
    }


def letter(grade):
    if grade < 0 or grade > 100:
        raise ValueError(f"grade out of range: {grade}")
    return "A" if grade >= 90 else "B" if grade >= 75 else "C" if grade >= 60 else "F"
