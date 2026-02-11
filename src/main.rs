fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_demonstrate_bdd_pattern() {
        // Given
        let input = "test input";
        let expected = "test input";

        // When
        let result = input;

        // Then
        assert_eq!(result, expected);
    }

    #[test]
    fn should_fail_when_condition_not_met() {
        // Given
        let value = 5;

        // When
        let is_even = value % 2 == 0;

        // Then
        assert!(!is_even, "5 should not be even");
    }
}
