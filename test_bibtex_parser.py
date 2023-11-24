import bibtex_parser

example_entries = [
    """@inproceedings{zhai2018autoencoder,
  title={Autoencoder and its various variants},
  author={Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang},
  booktitle={2018 IEEE international conference on systems, man, and cybernetics (SMC)},
  pages={415--419},
  year={2018},
  organization={IEEE}
}""",
    """ @article{schuetzke2023accelerating,
  title={Accelerating Materials Discovery: Automated Identification of Prospects from XRD Data in Fast Screening Experiments},
  author={Schuetzke, Jan and Schweidler, Simon, Münke, Friedrich R., Orth, André, Khandelwal, Anurag D., Aghassi-Hagmann, Jasmin, Breitung, Ben and Reischl, Markus},
  pages={12},
  year={2023},
}""",
    """   @article{suzuki2019automated,
  title={Automated estimation of materials parameter from X-ray absorption and electron energy-loss spectra with similarity measures},
  author={Suzuki, Yuta and Hino, Hideitsu and Kotsugi, Masato and Ono, Kanta},
  journal={Npj Computational Materials},
  volume={5},
  number={1},
  pages={39},
  year={2019},
  publisher={Nature Publishing Group UK London}
}""",
]

expected = [
    bibtex_parser.Entry(
        "inproceedings",
        "zhai2018autoencoder",
        {
            "title": "{Autoencoder and its various variants}",
            "author": "{Zhai, Junhai and Zhang, Sufang and Chen, Junfen and He, Qiang}",
            "booktitle": "{2018 IEEE international conference on systems, man, and cybernetics (SMC)}",
            "pages": "{415--419}",
            "year": "{2018}",
            "organization": "{IEEE}",
        },
    ),
    bibtex_parser.Entry(
        "article",
        "schuetzke2023accelerating",
        {
            "title": "{Accelerating Materials Discovery: Automated Identification of Prospects from XRD Data in Fast Screening Experiments}",
            "author": "{Schuetzke, Jan and Schweidler, Simon, Münke, Friedrich R., Orth, André, Khandelwal, Anurag D., Aghassi-Hagmann, Jasmin, Breitung, Ben and Reischl, Markus}",
            "pages": "{12}",
            "year": "{2023}",
        },
    ),
    bibtex_parser.Entry(
        "article",
        "suzuki2019automated",
        {
            "title": "{Automated estimation of materials parameter from X-ray absorption and electron energy-loss spectra with similarity measures}",
            "author": "{Suzuki, Yuta and Hino, Hideitsu and Kotsugi, Masato and Ono, Kanta}",
            "journal": "{Npj Computational Materials}",
            "volume": "{5}",
            "number": "{1}",
            "pages": "{39}",
            "year": "{2019}",
            "publisher": "{Nature Publishing Group UK London}",
        },
    ),
]


def test_parse_type():
    types = ["inproceedings", "article", "article"]
    for input, tipe in zip(example_entries, types):
        parser = bibtex_parser.Parser(input)
        parser.skip_whitespace()
        assert parser.parse_type() == tipe


def test_parse():
    for input, exp in zip(example_entries, expected):
        parser = bibtex_parser.Parser(input)
        parsed = parser.parse()
        assert parsed == exp

def test_parse_zhai():
    input = example_entries[0]
    parser = bibtex_parser.Parser(input)
    parsed = parser.parse()
    if isinstance(parsed, bibtex_parser.Error):
        print(parsed)
    assert parsed == expected[0]
