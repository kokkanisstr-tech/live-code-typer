using System;
using System.Collections.Generic;

abstract class CollegeMember
{
    public string FullName { get; set; }
    public int Id { get; set; }
    public int Age { get; set; }
    public string Email { get; set; }

    public abstract void ShowInfo();
}

class Student : CollegeMember
{
    public int CourseNumber { get; set; }
    public string Specialty { get; set; }
    public double Gpa { get; set; }

    public override void ShowInfo()
    {
        Console.WriteLine("Студент: " + FullName + ", ID " + Id + ", " + Age + " жас, " + Specialty + ", " + CourseNumber + " курс, GPA " + Gpa);
    }
}

class Teacher : CollegeMember
{
    public string Subject { get; set; }
    public int Experience { get; set; }
    public decimal Salary { get; set; }

    public override void ShowInfo()
    {
        Console.WriteLine("Оқытушы: " + FullName + ", ID " + Id + ", " + Age + " жас, " + Subject + ", өтіл " + Experience + " жыл, " + Salary + " тг");
    }
}

class Administrator : CollegeMember
{
    public string Department { get; set; }
    public string Position { get; set; }
    public decimal Salary { get; set; }

    public override void ShowInfo()
    {
        Console.WriteLine("Әкімші: " + FullName + ", ID " + Id + ", " + Age + " жас, " + Department + ", " + Position + ", " + Salary + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<CollegeMember> members = new List<CollegeMember>();
        members.Add(new Student { FullName = "Йусуф Кабиден", Id = 1, Age = 16, Email = "yusuf@mail.kz", CourseNumber = 2, Specialty = "ПО", Gpa = 3.7 });
        members.Add(new Student { FullName = "Ерлан Ардабек", Id = 2, Age = 17, Email = "erlan@mail.kz", CourseNumber = 2, Specialty = "ПО", Gpa = 3.4 });
        members.Add(new Student { FullName = "Дана Серик", Id = 3, Age = 17, Email = "dana@mail.kz", CourseNumber = 2, Specialty = "ПО", Gpa = 3.9 });
        members.Add(new Teacher { FullName = "Талдаубек Арай", Id = 4, Age = 35, Email = "arai@college.kz", Subject = "Информатика", Experience = 8, Salary = 280000 });
        members.Add(new Teacher { FullName = "Асатова Асель", Id = 5, Age = 40, Email = "asel@college.kz", Subject = "Программирование", Experience = 12, Salary = 300000 });
        members.Add(new Teacher { FullName = "Таңжарбаева Меруерт", Id = 6, Age = 33, Email = "meruert@college.kz", Subject = "Алгоритмы", Experience = 6, Salary = 250000 });
        members.Add(new Administrator { FullName = "Бекова Айгуль", Id = 7, Age = 45, Email = "aigul@college.kz", Department = "Оқу бөлімі", Position = "маман", Salary = 200000 });
        members.Add(new Administrator { FullName = "Смагулова Гульнар", Id = 8, Age = 50, Email = "gulnar@college.kz", Department = "Кітапхана", Position = "меңгеруші", Salary = 180000 });
        members.Add(new Administrator { FullName = "Ахметова Динара", Id = 9, Age = 29, Email = "dinara@college.kz", Department = "Кадрлар", Position = "инспектор", Salary = 170000 });

        int students = 0;
        int teachers = 0;
        int admins = 0;
        foreach (CollegeMember member in members)
        {
            member.ShowInfo();
            if (member.Age < 18)
            {
                Console.WriteLine("  18 жасқа толмаған");
            }
            Student student = member as Student;
            if (student != null)
            {
                students = students + 1;
                if (student.Gpa > 3.5)
                {
                    Console.WriteLine("  GPA 3.5-тен жоғары");
                }
            }
            Teacher teacher = member as Teacher;
            if (teacher != null)
            {
                teachers = teachers + 1;
                if (teacher.Experience > 10)
                {
                    Console.WriteLine("  өтілі 10 жылдан асады");
                }
            }
            Administrator admin = member as Administrator;
            if (admin != null)
            {
                admins = admins + 1;
                Console.WriteLine("  жалақысы: " + admin.Salary + " тг");
            }
        }
        Console.WriteLine("Студенттер: " + students);
        Console.WriteLine("Оқытушылар: " + teachers);
        Console.WriteLine("Әкімшілер: " + admins);
        Console.WriteLine("Барлығы: " + members.Count);
    }
}
