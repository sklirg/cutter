.PHONY: build create deploy invoke prepare

build:
	@docker run --rm -it -v $(shell pwd):/home/rust/src ekidd/rust-musl-builder cargo build --release --bin cutter-lambda --target x86_64-unknown-linux-musl

create:
	@aws lambda create-function \
	--function-name $(CUTTER_AWS_LAMBDA_NAME) \
	--handler doesnt.matter \
	--zip-file fileb://./lambda.zip \
	--runtime provided \
	--role $(CUTTER_AWS_IAM_ROLE) \
	--environment Variables={RUST_BACKTRACE=1} \
	--tracing-config Mode=Active
	# Add timeout

deploy:
	@aws lambda update-function-code \
	--function-name $(CUTTER_AWS_LAMBDA_NAME) \
	--zip-file fileb://./lambda.zip

invoke:
	@aws lambda invoke --function-name cutter:1 \
	--payload '{"bucket": "camerabag", "prefix": "77954ebc-11d8-4628-adeb-cdadd5027c49"}' \
	--invocation-type Event \
	output.json

prepare:
	@cp target/x86_64-unknown-linux-musl/release/cutter-lambda bootstrap && \
	zip lambda.zip bootstrap
