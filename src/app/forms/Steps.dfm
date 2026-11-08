object StepsFrm: TStepsFrm
  Left = 240
  Top = 180
  BorderStyle = bsDialog
  Caption = '@MB:StepsAndNoise'
  ClientHeight = 420
  ClientWidth = 520
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object PostCheck: TCheckBox
    Left = 12
    Top = 12
    Width = 496
    Height = 30
    Caption = '@MB:OwnStepSettingsForThePasses'
    WordWrap = True
    TabOrder = 0
    OnClick = PostCheckClick
  end
  object RayLabel: TLabel
    Left = 30
    Top = 52
    Width = 170
    Height = 13
    Caption = '@MB:RayStepOfThePostPasses'
  end
  object RayEdit: TEdit
    Left = 210
    Top = 48
    Width = 60
    Height = 21
    TabOrder = 1
    OnChange = PostEditChange
  end
  object DeLabel: TLabel
    Left = 30
    Top = 78
    Width = 170
    Height = 13
    Caption = '@MB:DEStopOfThePostPasses'
  end
  object DeEdit: TEdit
    Left = 210
    Top = 74
    Width = 60
    Height = 21
    TabOrder = 2
    OnChange = PostEditChange
  end
  object PostNote: TLabel
    Left = 290
    Top = 48
    Width = 218
    Height = 80
    AutoSize = False
    WordWrap = True
    Caption = '@MB:TheMainPassCanMarchWith'
  end
  object Bevel1: TBevel
    Left = 12
    Top = 138
    Width = 496
    Height = 2
  end
  object DoctorLabel: TLabel
    Left = 12
    Top = 148
    Width = 100
    Height = 16
    Caption = '@MB:NoiseDoctor'
    Font.Charset = DEFAULT_CHARSET
    Font.Color = clWindowText
    Font.Height = -13
    Font.Name = 'MS Sans Serif'
    Font.Style = [fsBold]
    ParentFont = False
  end
  object DoctorNote: TLabel
    Left = 12
    Top = 168
    Width = 496
    Height = 32
    AutoSize = False
    WordWrap = True
    Caption = '@MB:RendersTheMiddleOfThePicture'
  end
  object RunBtn: TButton
    Left = 12
    Top = 206
    Width = 180
    Height = 25
    Caption = '@MB:RunTheNoiseDoctor'
    TabOrder = 3
    OnClick = RunBtnClick
  end
  object ApplyBtn: TButton
    Left = 200
    Top = 206
    Width = 200
    Height = 25
    Caption = '@MB:ApplyTheRecommendedSetting'
    Enabled = False
    TabOrder = 4
    OnClick = ApplyBtnClick
  end
  object ResultMemo: TMemo
    Left = 12
    Top = 240
    Width = 496
    Height = 136
    ReadOnly = True
    ScrollBars = ssVertical
    TabOrder = 5
  end
  object CloseBtn: TButton
    Left = 418
    Top = 384
    Width = 90
    Height = 26
    Caption = '@MB:Close'
    TabOrder = 6
    OnClick = CloseBtnClick
  end
  object Timer1: TTimer
    Enabled = False
    Interval = 200
    OnTimer = Timer1Timer
    Left = 460
    Top = 150
  end
end
