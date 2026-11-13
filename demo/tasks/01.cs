using System;
using System.Collections.Generic;

abstract class Lesson
{
    public string SubjectName { get; set; }
    public string TeacherName { get; set; }
    public int Duration { get; set; }
    public string RoomNumber { get; set; }

    public abstract void ShowDetails();
}

class Lecture : Lesson
{
    public override void ShowDetails()
    {
        Console.WriteLine("Дәріс | " + SubjectName + " | " + TeacherName + " | " + RoomNumber + " | " + Duration + " мин");
    }
}

class Laboratory : Lesson
{
    public override void ShowDetails()
    {
        Console.WriteLine("Зертхана | " + SubjectName + " | " + TeacherName + " | " + RoomNumber + " | " + Duration + " мин");
    }
}

class PracticeLesson : Lesson
{
    public override void ShowDetails()
    {
        Console.WriteLine("Практика | " + SubjectName + " | " + TeacherName + " | " + RoomNumber + " | " + Duration + " мин");
    }
}

class Program
{
    static void Main()
    {
        List<Lesson> lessons = new List<Lesson>();
        lessons.Add(new Lecture { SubjectName = "Математика", TeacherName = "Ахметова А.", Duration = 90, RoomNumber = "101" });
        lessons.Add(new Lecture { SubjectName = "Физика", TeacherName = "Сериков Б.", Duration = 80, RoomNumber = "102" });
        lessons.Add(new Laboratory { SubjectName = "Химия", TeacherName = "Оспанова Г.", Duration = 120, RoomNumber = "305" });
        lessons.Add(new Laboratory { SubjectName = "Биология", TeacherName = "Нурланова Д.", Duration = 90, RoomNumber = "306" });
        lessons.Add(new PracticeLesson { SubjectName = "Информатика", TeacherName = "Талдаубек А.", Duration = 90, RoomNumber = "210" });
        lessons.Add(new PracticeLesson { SubjectName = "Английский", TeacherName = "Утепкалиева А.", Duration = 60, RoomNumber = "115" });

        foreach (Lesson lesson in lessons)
        {
            lesson.ShowDetails();
        }

        Console.WriteLine("--- 80 минуттан узак сабактар ---");
        foreach (Lesson lesson in lessons)
        {
            if (lesson.Duration > 80)
            {
                Console.WriteLine(lesson.SubjectName + " - " + lesson.Duration + " мин");
            }
        }
    }
}
