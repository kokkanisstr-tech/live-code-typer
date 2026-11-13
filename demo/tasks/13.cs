using System;
using System.Collections.Generic;

abstract class Competition
{
    public string CompetitionName { get; set; }
    public int ParticipantCount { get; set; }
    public decimal EntryFee { get; set; }
    public int MaxParticipants { get; set; }

    public abstract void ShowRules();
}

class RunningCompetition : Competition
{
    public double Distance { get; set; }

    public override void ShowRules()
    {
        Console.WriteLine("Жүгіру: " + CompetitionName + ", " + Distance + " км, жарна " + EntryFee + " тг");
    }
}

class SwimmingCompetition : Competition
{
    public double PoolLength { get; set; }

    public override void ShowRules()
    {
        Console.WriteLine("Жүзу: " + CompetitionName + ", бассейн " + PoolLength + " м, жарна " + EntryFee + " тг");
    }
}

class CyclingCompetition : Competition
{
    public double RouteLength { get; set; }

    public override void ShowRules()
    {
        Console.WriteLine("Веложарыс: " + CompetitionName + ", бағыт " + RouteLength + " км, жарна " + EntryFee + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<Competition> competitions = new List<Competition>();
        competitions.Add(new RunningCompetition { CompetitionName = "Көктем жүгірісі", ParticipantCount = 120, EntryFee = 4000, MaxParticipants = 100, Distance = 5 });
        competitions.Add(new RunningCompetition { CompetitionName = "Марафон", ParticipantCount = 80, EntryFee = 6000, MaxParticipants = 200, Distance = 42 });
        competitions.Add(new SwimmingCompetition { CompetitionName = "Су спорт", ParticipantCount = 60, EntryFee = 3500, MaxParticipants = 50, PoolLength = 25 });
        competitions.Add(new SwimmingCompetition { CompetitionName = "Теңіз кубогы", ParticipantCount = 40, EntryFee = 5500, MaxParticipants = 80, PoolLength = 50 });
        competitions.Add(new CyclingCompetition { CompetitionName = "Вело-Астана", ParticipantCount = 150, EntryFee = 4500, MaxParticipants = 150, RouteLength = 60 });
        competitions.Add(new CyclingCompetition { CompetitionName = "Тау велосипеді", ParticipantCount = 70, EntryFee = 7000, MaxParticipants = 90, RouteLength = 30 });

        foreach (Competition competition in competitions)
        {
            competition.ShowRules();
            if (competition.ParticipantCount > competition.MaxParticipants)
            {
                Console.WriteLine("  қатысушылар шектен асты");
            }
            if (competition.EntryFee < 5000)
            {
                Console.WriteLine("  жарна 5 000 теңгеден төмен");
            }
            decimal totalFee = competition.ParticipantCount * competition.EntryFee;
            Console.WriteLine("  жиналған жарна: " + totalFee + " тг");
        }
    }
}
