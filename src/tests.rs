mod tests {
    use dockertest::{
        waitfor::{MessageSource, MessageWait},
        Composition, DockerTest,
    };

    const IMAGE_NAME: &str = "crust";
    const DEFAULT_PORT: u32 = 8000;

    #[test]
    fn test_immediate_successor() {
        let mut test = DockerTest::new();
        let num_containers = 5;
        for i in 0..num_containers {
            let container_name = format!("{}{}", IMAGE_NAME, i);
            let mut node = Composition::with_repository(IMAGE_NAME)
                .with_container_name(container_name)
                .with_wait_for(Box::new(MessageWait {
                    message: format!("Listening for requests at http://0.0.0.0:{}", DEFAULT_PORT),
                    source: MessageSource::Stdout,
                    timeout: 60,
                }));
            let port_on_host: u32 = DEFAULT_PORT + i;
            node.port_map(DEFAULT_PORT, port_on_host);
            test.add_composition(node);
        }
        test.run(|_ops| async move {
            for i in 0..num_containers {
                let resp0 =
                    reqwest::get(format!("http://localhost:{}/successor", DEFAULT_PORT + i))
                        .await
                        .unwrap();
                let result = resp0.text().await.unwrap();
                let next_num = if i == num_containers - 1 { 0 } else { i + 1 };
                let expected_successor = format!("node{}", next_num);
                assert_eq!(
                    result, expected_successor,
                    "node{} reported an incorrect successor (actual=left, expected=right)",
                    i
                );
            }
        });
    }
}
