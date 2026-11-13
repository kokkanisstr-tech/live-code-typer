using System;
using System.Collections.Generic;

abstract class Course
{
    public string CourseName { get; set; }
    public int Duration { get; set; }
    public decimal Price { get; set; }
    public int StudentCount { get; set; }

    public abstract void ShowProgram();
}

class ProgrammingCourse : Course
{
    public string ProgrammingLanguage { get; set; }

    public override void ShowProgram()
    {
        Console.WriteLine("Бағдарламалау курсы: " + CourseName + ", тіл " + ProgrammingLanguage + ", " + Duration + " сағат, " + Price + " тг");
    }
}

class DesignCourse : Course
{
    public string DesignTool { get; set; }

    public override void ShowProgram()
    {
        Console.WriteLine("Дизайн курсы: " + CourseName + ", құрал " + DesignTool + ", " + Duration + " сағат, " + Price + " тг");
    }
}

class LanguageCourse : Course
{
    public string LanguageName { get; set; }

    public override void ShowProgram()
    {
        Console.WriteLine("Тіл курсы: " + CourseName + ", " + LanguageName + ", " + Duration + " сағат, " + Price + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<Course> courses = new List<Course>();
        courses.Add(new ProgrammingCourse { CourseName = "Python негіздері", Duration = 40, Price = 45000, StudentCount = 25, ProgrammingLanguage = "Python" });
        courses.Add(new ProgrammingCourse { CourseName = "C# практикасы", Duration = 60, Price = 60000, StudentCount = 18, ProgrammingLanguage = "C#" });
        courses.Add(new DesignCourse { CourseName = "Figma дизайн", Duration = 30, Price = 35000, StudentCount = 22, DesignTool = "Figma" });
        courses.Add(new DesignCourse { CourseName = "Photoshop", Duration = 45, Price = 48000, StudentCount = 14, DesignTool = "Photoshop" });
        courses.Add(new LanguageCourse { CourseName = "Ағылшын тілі", Duration = 80, Price = 52000, StudentCount = 30, LanguageName = "ағылшын" });
        courses.Add(new LanguageCourse { CourseName = "Қазақ тілі", Duration = 20, Price = 25000, StudentCount = 12, LanguageName = "қазақ" });

        int students = 0;
        decimal total = 0;
        foreach (Course course in courses)
        {
            course.ShowProgram();
            students = students + course.StudentCount;
            total = total + course.Price;
            if (course.Duration > 40)
            {
                Console.WriteLine("  узактыгы 40 сағаттан асады");
            }
            if (course.Price < 50000)
            {
                Console.WriteLine("  бағасы 50 000 теңгеден төмен");
            }
        }
        Console.WriteLine("Студенттердің жалпы саны: " + students);
        Console.WriteLine("Курстардың жалпы бағасы: " + total + " тг");
    }
}
