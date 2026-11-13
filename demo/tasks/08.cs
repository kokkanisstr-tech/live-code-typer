using System;
using System.Collections.Generic;

abstract class Storage
{
    public string StorageName { get; set; }
    public double Capacity { get; set; }
    public double UsedSpace { get; set; }
    public bool IsConnected { get; set; }

    public abstract bool SaveFile(double fileSize);
    public abstract void ShowStorageInfo();

    protected bool HasSpace(double fileSize)
    {
        return Capacity - UsedSpace >= fileSize;
    }
}

class LocalStorage : Storage
{
    public string DriveLetter { get; set; }

    public override bool SaveFile(double fileSize)
    {
        if (fileSize <= 0 || HasSpace(fileSize) == false)
        {
            return false;
        }
        UsedSpace = UsedSpace + fileSize;
        return true;
    }

    public override void ShowStorageInfo()
    {
        Console.WriteLine(StorageName + " (" + DriveLetter + "), бос орын " + (Capacity - UsedSpace) + " ГБ");
    }
}

class CloudStorage : Storage
{
    public string ProviderName { get; set; }

    public override bool SaveFile(double fileSize)
    {
        if (fileSize <= 0 || IsConnected == false || HasSpace(fileSize) == false)
        {
            return false;
        }
        UsedSpace = UsedSpace + fileSize;
        return true;
    }

    public override void ShowStorageInfo()
    {
        Console.WriteLine(StorageName + " (" + ProviderName + "), бос орын " + (Capacity - UsedSpace) + " ГБ");
    }
}

class ExternalDrive : Storage
{
    public string ConnectionType { get; set; }

    public override bool SaveFile(double fileSize)
    {
        if (fileSize <= 0 || IsConnected == false || HasSpace(fileSize) == false)
        {
            return false;
        }
        UsedSpace = UsedSpace + fileSize;
        return true;
    }

    public override void ShowStorageInfo()
    {
        Console.WriteLine(StorageName + " (" + ConnectionType + "), бос орын " + (Capacity - UsedSpace) + " ГБ");
    }
}

class Program
{
    static void Main()
    {
        List<Storage> storages = new List<Storage>();
        storages.Add(new LocalStorage { StorageName = "C", Capacity = 500, UsedSpace = 480, IsConnected = true, DriveLetter = "C:" });
        storages.Add(new LocalStorage { StorageName = "D", Capacity = 1000, UsedSpace = 200, IsConnected = true, DriveLetter = "D:" });
        storages.Add(new CloudStorage { StorageName = "Google Drive", Capacity = 100, UsedSpace = 10, IsConnected = true, ProviderName = "Google" });
        storages.Add(new CloudStorage { StorageName = "OneDrive", Capacity = 50, UsedSpace = 5, IsConnected = false, ProviderName = "Microsoft" });
        storages.Add(new ExternalDrive { StorageName = "USB", Capacity = 64, UsedSpace = 60, IsConnected = true, ConnectionType = "USB 3.0" });
        storages.Add(new ExternalDrive { StorageName = "HDD", Capacity = 2000, UsedSpace = 100, IsConnected = true, ConnectionType = "SATA" });

        double[] files = { 5, 15, 1, 30, 2, 50 };
        for (int i = 0; i < storages.Count; i++)
        {
            bool saved = storages[i].SaveFile(files[i]);
            storages[i].ShowStorageInfo();
            if (saved)
            {
                Console.WriteLine("  " + files[i] + " ГБ файл сақталды");
            }
            else
            {
                Console.WriteLine("  " + files[i] + " ГБ файл сақталмады");
            }
        }

        Console.WriteLine("--- Бос орны 10 ГБ-тан аз ---");
        foreach (Storage storage in storages)
        {
            if (storage.Capacity - storage.UsedSpace < 10)
            {
                Console.WriteLine(storage.StorageName);
            }
        }
    }
}
