use bibextract::extract_survey_internal;
use mockito::{Matcher, Server};
use std::io::Write;
use tokio::runtime::Runtime;

const MOCK_LATEX_CONTENT: &str = r#"\documentclass{article}
\begin{document}
\title{Deep Learning Survey}
\author{John Doe and Jane Smith}

\section{Related Work}
Deep learning has been extensively studied \cite{lecun2015deep,goodfellow2016deep}.
Recent advances include transformers \cite{vaswani2017attention}.

\subsection{Background}
Foundation models \cite{bommasani2021opportunities} have revolutionized the field.

\section{Methodology}
Our approach builds on existing work.

\bibliography{refs}
\end{document}"#;

const MOCK_BBL_CONTENT: &str = r#"\begin{thebibliography}{99}

\bibitem{lecun2015deep}
Yann LeCun, Yoshua Bengio, and Geoffrey Hinton.
\newblock Deep learning.
\newblock {\em Nature}, 521(7553):436--444, 2015.

\bibitem{goodfellow2016deep}
Ian Goodfellow, Yoshua Bengio, and Aaron Courville.
\newblock Deep learning.
\newblock MIT press, 2016.

\bibitem{vaswani2017attention}
Ashish Vaswani et~al.
\newblock Attention is all you need.
\newblock In {\em Advances in neural information processing systems}, pages 5998--6008, 2017.

\bibitem{bommasani2021opportunities}
Rishi Bommasani et~al.
\newblock On the opportunities and risks of foundation models.
\newblock arXiv preprint arXiv:2108.07258, 2021.

\end{thebibliography}"#;

fn create_mock_tar_gz() -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    
    let mut tar_data = Vec::new();
    {
        let mut tar = Builder::new(&mut tar_data);
        
        // Add main.tex
        let mut header = tar::Header::new_gnu();
        header.set_path("2104.08653/main.tex").unwrap();
        header.set_size(MOCK_LATEX_CONTENT.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, MOCK_LATEX_CONTENT.as_bytes()).unwrap();
        
        // Add main.bbl
        let mut header = tar::Header::new_gnu();
        header.set_path("2104.08653/main.bbl").unwrap();
        header.set_size(MOCK_BBL_CONTENT.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, MOCK_BBL_CONTENT.as_bytes()).unwrap();
        
        tar.finish().unwrap();
    }
    
    let mut gz_data = Vec::new();
    {
        let mut encoder = GzEncoder::new(&mut gz_data, Compression::default());
        encoder.write_all(&tar_data).unwrap();
        encoder.finish().unwrap();
    }
    
    gz_data
}

#[test]
fn test_full_pipeline_with_mocked_apis() {
    // Create a single runtime for the entire test
    let rt = Runtime::new().unwrap();
    
    rt.block_on(async {
        let mut server = Server::new_async().await;
        
        // Mock arXiv source download (allow multiple calls)
        let mock_tar = create_mock_tar_gz();
        let arxiv_source_mock = server
            .mock("GET", "/e-print/2104.08653")
            .with_status(200)
            .with_header("content-type", "application/gzip")
            .with_body(mock_tar)
            .expect_at_least(1)
            .create_async()
            .await;
        
        // Mock arXiv BibTeX API calls
        let _lecun_bibtex = r#"@article{lecun2015deep,
  title={Deep learning},
  author={LeCun, Yann and Bengio, Yoshua and Hinton, Geoffrey},
  journal={Nature},
  volume={521},
  number={7553},
  pages={436--444},
  year={2015},
  publisher={Nature Publishing Group}
}"#;
        
        let bommasani_bibtex = r#"@article{bommasani2021opportunities,
  title={On the opportunities and risks of foundation models},
  author={Bommasani, Rishi and Hudson, Drew A and Adeli, Ehsan and Altman, Russ and Arora, Sanjeev and von Arx, Sydney and Bernstein, Michael S and Bohg, Jeannette and Bosselut, Antoine and Brunskill, Emma and others},
  journal={arXiv preprint arXiv:2108.07258},
  year={2021}
}"#;
        
        let _arxiv_bibtex_bommasani = server
            .mock("GET", "/bibtex/2108.07258")
            .with_status(200)
            .with_body(bommasani_bibtex)
            .create_async()
            .await;
        
        // Mock the BibTeX for the test paper ID (2104.08653)
        let test_paper_bibtex = r#"@article{testpaper2021,
  title={Test Paper for Integration Tests},
  author={Test Author One and Test Author Two},
  journal={Test Journal},
  year={2021}
}"#;
        
        let _arxiv_bibtex_test_paper = server
            .mock("GET", "/bibtex/2104.08653")
            .with_status(200)
            .with_body(test_paper_bibtex)
            .create_async()
            .await;
        
        // Mock DBLP API calls
        let dblp_lecun_response = r#"{
  "result": {
    "hits": {
      "@total": "1",
      "hit": [{
        "info": {
          "title": "Deep learning",
          "authors": {
            "author": [
              {"text": "Yann LeCun"},
              {"text": "Yoshua Bengio"},
              {"text": "Geoffrey Hinton"}
            ]
          },
          "venue": "Nature",
          "volume": "521",
          "year": "2015",
          "url": "https://doi.org/10.1038/nature14539",
          "doi": "10.1038/nature14539"
        }
      }]
    }
  }
}"#;
        
        let dblp_vaswani_response = r#"{
  "result": {
    "hits": {
      "@total": "1",
      "hit": [{
        "info": {
          "title": "Attention is all you need",
          "authors": {
            "author": [
              {"text": "Ashish Vaswani"},
              {"text": "Noam Shazeer"},
              {"text": "Niki Parmar"}
            ]
          },
          "venue": "NIPS",
          "year": "2017",
          "url": "https://proceedings.neurips.cc/paper/2017/hash/3f5ee243547dee91fbd053c1c4a845aa-Abstract.html"
        }
      }]
    }
  }
}"#;
        
        let dblp_goodfellow_response = r#"{
  "result": {
    "hits": {
      "@total": "1", 
      "hit": [{
        "info": {
          "title": "Deep Learning",
          "authors": {
            "author": [
              {"text": "Ian Goodfellow"},
              {"text": "Yoshua Bengio"},
              {"text": "Aaron Courville"}
            ]
          },
          "venue": "MIT Press",
          "year": "2016",
          "url": "https://www.deeplearningbook.org/"
        }
      }]
    }
  }
}"#;
        
        let _dblp_lecun_mock = server
            .mock("GET", "/search/publ/api")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("q".into(), "Deep+learning".into()),
                Matcher::UrlEncoded("format".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(dblp_lecun_response)
            .create_async()
            .await;
        
        let _dblp_vaswani_mock = server
            .mock("GET", "/search/publ/api")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("q".into(), "Attention+is+all+you+need".into()),
                Matcher::UrlEncoded("format".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(dblp_vaswani_response)
            .create_async()
            .await;
        
        let _dblp_goodfellow_mock = server
            .mock("GET", "/search/publ/api")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("q".into(), "Deep+learning".into()),
                Matcher::UrlEncoded("format".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(dblp_goodfellow_response)
            .create_async()
            .await;
        
        // Catch-all DBLP mock for any other queries
        let _dblp_catchall_mock = server
            .mock("GET", "/search/publ/api")
            .match_query(Matcher::UrlEncoded("format".into(), "json".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result": {"hits": {"@total": "0"}}}"#)
            .expect_at_least(0)
            .create_async()
            .await;
        
        // Override the base URLs for testing
        std::env::set_var("ARXIV_BASE_URL", &server.url());
        std::env::set_var("DBLP_BASE_URL", &server.url());
        std::env::set_var("API_TIMEOUT_SECS", "10"); // Shorter timeout for faster tests
        
        // Execute the full pipeline in an async context
        let paper_ids = vec!["2104.08653".to_string()];
        let result = extract_survey_internal(paper_ids).await;
        
        // Verify the pipeline executed successfully
        assert!(result.is_ok(), "Pipeline should succeed with mocked APIs: {:?}", result.err());
        
        let (survey_text, bibtex) = result.unwrap();
        
        // Verify survey content contains expected sections
        assert!(survey_text.contains("\\section{Related Work}"), "Should contain Related Work section");
        assert!(survey_text.contains("\\section{Background}"), "Should contain Background section");
        assert!(survey_text.contains("Deep learning has been extensively studied"), "Should contain section content");
        
        // Verify citations are present and normalized
        assert!(survey_text.contains("\\cite{"), "Should contain citation commands");
        
        // Verify BibTeX output contains verified entries
        assert!(bibtex.contains("@article{"), "Should contain article entries");
        assert!(bibtex.contains("title = {"), "Should contain titles");
        assert!(bibtex.contains("author = {"), "Should contain author information");
        assert!(bibtex.contains("year = {"), "Should contain publication years");
        
        // Verify mocks were called
        arxiv_source_mock.assert_async().await;
        
        // Clean up environment variables
        std::env::remove_var("ARXIV_BASE_URL");
        std::env::remove_var("DBLP_BASE_URL");
        std::env::remove_var("API_TIMEOUT_SECS");
    });
}

#[test]
fn test_pipeline_with_network_failures() {
    let rt = Runtime::new().unwrap();
    
    rt.block_on(async {
        let mut server = Server::new_async().await;
        
        // Mock arXiv source download (successful)
        let mock_tar = create_mock_tar_gz();
        let _arxiv_source_mock = server
            .mock("GET", "/e-print/2104.08653")
            .with_status(200)
            .with_header("content-type", "application/gzip")
            .with_body(mock_tar)
            .create_async()
            .await;
        
        // Mock failing BibTeX API calls
        let _arxiv_bibtex_fail = server
            .mock("GET", "/bibtex/2108.07258")
            .with_status(404)
            .create_async()
            .await;
        
        // But the main paper BibTeX must succeed for the pipeline to work
        let test_paper_bibtex = r#"@article{testpaper2021,
  title={Test Paper for Network Failure Test},
  author={Test Author One and Test Author Two},
  journal={Test Journal},
  year={2021}
}"#;
        
        let _arxiv_bibtex_main_paper = server
            .mock("GET", "/bibtex/2104.08653")
            .with_status(200)
            .with_body(test_paper_bibtex)
            .create_async()
            .await;
        
        // Mock failing DBLP API calls
        let _dblp_fail_mock = server
            .mock("GET", "/search/publ/api")
            .with_status(500)
            .create_async()
            .await;
        
        std::env::set_var("ARXIV_BASE_URL", &server.url());
        std::env::set_var("DBLP_BASE_URL", &server.url());
        std::env::set_var("API_TIMEOUT_SECS", "2"); // Short timeout for faster test execution
        
        let paper_ids = vec!["2104.08653".to_string()];
        let result = extract_survey_internal(paper_ids).await;
        
        // Pipeline should still succeed even with API failures
        assert!(result.is_ok(), "Pipeline should handle API failures gracefully");
        
        let (survey_text, bibtex) = result.unwrap();
        
        // Should still extract sections and generate output
        assert!(survey_text.contains("\\section{Related Work}"), "Should extract sections despite API failures");
        assert!(!bibtex.is_empty(), "Should generate some BibTeX output");
        
        // Clean up
        std::env::remove_var("ARXIV_BASE_URL");
        std::env::remove_var("DBLP_BASE_URL");
        std::env::remove_var("API_TIMEOUT_SECS");
    });
}

#[tokio::test]
async fn test_invalid_paper_id() {
    let paper_ids = vec!["invalid-id".to_string()];
    let result = extract_survey_internal(paper_ids).await;
    
    assert!(result.is_err(), "Should fail with invalid paper ID");
}

#[tokio::test]
async fn test_empty_paper_ids() {
    let paper_ids = vec![];
    let result = extract_survey_internal(paper_ids).await;
    
    assert!(result.is_err(), "Should fail with empty paper IDs");
}
