error_chain! {
    errors {
        ToDo

        Unexpected {
            description("Unexpected error occured")
        }

        Timeout

        NoTenant

        NoResources

        EmptyDocument

        EmptyDelta

        DeltaWithoutOperator {
            description("Delta statement doesn't have an operator (was a triple)")
        }

        OperatorWithoutGraphName {
            description("Operator did not specify a graph parameter")
        }

        InvalidGraphFormat {
            description("Operator graph parameter not properly formatted")
        }

        ParserError(t: String) {
            description("Error parsing message")
            display("Parser error: {}", t)
        }

        Commit {
            description("Failed to commit message after processing")
        }

        SecurityError(t: String) {
            description("Security error")
            display("Security error: {}", t)
        }
    }
}
